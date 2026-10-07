use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use walkdir::WalkDir;

type ScanTarget = (PathBuf, String);

// Discovering cache roots beneath application-data directories can involve a
// substantial directory walk.  The roots are stable for the lifetime of an
// app session, so retain the safe inventory and reuse it for both scanning and
// deletion validation.  The contents of each root are still read afresh by
// the cleaner scan.
static CLEANABLE_TARGETS: OnceLock<Vec<ScanTarget>> = OnceLock::new();

/// Build a child process command that cannot flash a console window when the
/// desktop application is running on Windows. GUI applications do not own a
/// console, so every `powershell.exe`/`cmd.exe` child otherwise creates one.
pub fn background_command(program: impl AsRef<OsStr>) -> Command {
    #[cfg(target_os = "windows")]
    let mut command = Command::new(program);

    #[cfg(not(target_os = "windows"))]
    let command = Command::new(program);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        // CREATE_NO_WINDOW
        command.creation_flags(0x0800_0000);
    }

    command
}

pub fn background_powershell_command() -> Command {
    let mut command = background_command("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
    ]);
    command
}

/// Returns user-owned directories commonly used for cache, logs, and temp
/// files. The root itself is never deletable; only selected children are.
///
/// The cache directory comes from the platform's native location (`XDG`, known
/// Windows folders, or `~/Library/Caches`) instead of assuming English folder
/// names. We also discover common nested cache directories used by sandboxed
/// and browser applications.
pub fn cleanable_targets() -> Vec<ScanTarget> {
    CLEANABLE_TARGETS
        .get_or_init(discover_cleanable_targets)
        .clone()
}

fn discover_cleanable_targets() -> Vec<ScanTarget> {
    let mut targets = Vec::new();
    let mut target_identities = HashSet::new();

    add_target(
        &mut targets,
        &mut target_identities,
        dirs::cache_dir(),
        "Cache",
    );

    #[cfg(target_os = "macos")]
    {
        let home = home_dir();
        add_target(
            &mut targets,
            &mut target_identities,
            Some(home.join("Library").join("Logs")),
            "Log",
        );
        add_target(
            &mut targets,
            &mut target_identities,
            Some(home.join("Library").join("WebKit")),
            "Web Cache",
        );

        for root in [
            home.join("Library").join("Application Support"),
            home.join("Library").join("Containers"),
            home.join("Library").join("Group Containers"),
        ] {
            add_nested_cache_targets(&mut targets, &mut target_identities, &root, 6);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home = home_dir();
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local").join("share"));

        add_target(
            &mut targets,
            &mut target_identities,
            Some(data_home.join("Trash").join("files")),
            "Trash",
        );
        add_target(
            &mut targets,
            &mut target_identities,
            Some(data_home.join("Trash").join("info")),
            "Trash metadata",
        );
        add_nested_cache_targets(&mut targets, &mut target_identities, &data_home, 5);
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(local_data) = dirs::data_local_dir() {
            add_target(
                &mut targets,
                &mut target_identities,
                Some(local_data.join("Temp")),
                "Temp",
            );
            add_target(
                &mut targets,
                &mut target_identities,
                Some(
                    local_data
                        .join("Microsoft")
                        .join("Windows")
                        .join("INetCache"),
                ),
                "Internet Cache",
            );
            add_target(
                &mut targets,
                &mut target_identities,
                Some(local_data.join("CrashDumps")),
                "Crash Dump",
            );

            // Store apps keep their cache beneath LocalAppData\Packages, and
            // Chromium-based apps commonly use Cache/Code Cache/GPUCache.
            add_nested_cache_targets(&mut targets, &mut target_identities, &local_data, 5);
        }

        // On Windows this normally resolves to the user's LocalAppData\Temp.
        // Do not use the process temp directory on Linux, where it is often
        // the shared /tmp directory.
        add_target(
            &mut targets,
            &mut target_identities,
            Some(std::env::temp_dir()),
            "Temp",
        );
    }

    targets
}

/// Return all standard user content directories plus visible custom folders at
/// the top level of the profile. `dirs` handles XDG configuration on Linux and
/// redirected Known Folders/OneDrive on Windows.
///
/// We intentionally do not recursively start at the profile root: that would
/// pull in hidden application data (`Library`, `AppData`, `.local`) and make a
/// routine file scan unexpectedly slow. Every visible user folder is still
/// searched recursively, including folders created by the user.
pub fn user_scannable_dirs() -> Vec<(PathBuf, String)> {
    let mut targets = Vec::new();
    let mut target_identities = HashSet::new();

    for (path, label) in [
        (dirs::desktop_dir(), "Desktop"),
        (dirs::download_dir(), "Downloads"),
        (dirs::document_dir(), "Documents"),
        (dirs::picture_dir(), "Pictures"),
        (dirs::audio_dir(), "Music"),
        (dirs::video_dir(), "Videos"),
        (dirs::public_dir(), "Public"),
    ] {
        add_target(&mut targets, &mut target_identities, path, label);
    }

    let home = home_dir();
    if let Ok(entries) = fs::read_dir(&home) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().trim().to_owned();

            if name.is_empty()
                || name.starts_with('.')
                || should_skip_profile_directory(&name)
                || !path.is_dir()
            {
                continue;
            }

            add_target(&mut targets, &mut target_identities, Some(path), name);
        }
    }

    targets
}

fn should_skip_profile_directory(name: &str) -> bool {
    #[cfg(target_os = "macos")]
    if name == "Library" {
        return true;
    }

    #[cfg(target_os = "windows")]
    if matches!(name, "AppData" | "Application Data" | "Local Settings") {
        return true;
    }

    #[cfg(target_os = "linux")]
    if matches!(name, "snap" | ".local") {
        return true;
    }

    false
}

fn is_cache_directory_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "cache"
            | "caches"
            | "code cache"
            | "gpucache"
            | "cachedata"
            | "cachestorage"
            | "localcache"
            | "inetcache"
            | "tempstate"
    )
}

fn add_nested_cache_targets(
    targets: &mut Vec<ScanTarget>,
    target_identities: &mut HashSet<PathBuf>,
    root: &Path,
    max_depth: usize,
) {
    if !root.is_dir() {
        return;
    }

    let mut walker = WalkDir::new(root)
        .follow_links(false)
        .same_file_system(true)
        .min_depth(1)
        .max_depth(max_depth)
        .into_iter();

    while let Some(entry) = walker.next() {
        let Ok(entry) = entry else {
            continue;
        };

        if !entry.file_type().is_dir() {
            continue;
        }

        if !is_cache_directory_name(&entry.file_name().to_string_lossy()) {
            continue;
        }

        // Keep every matching root, including nested cache directories, so
        // users can inspect and remove the specific cache they chose.
        add_target(targets, target_identities, Some(entry.into_path()), "Cache");
    }
}

fn path_identity(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn add_target(
    targets: &mut Vec<ScanTarget>,
    target_identities: &mut HashSet<PathBuf>,
    path: Option<PathBuf>,
    label: impl Into<String>,
) {
    let Some(path) = path else {
        return;
    };

    if !path.is_dir() {
        return;
    }

    let identity = path_identity(&path);
    if !target_identities.insert(identity) {
        return;
    }

    targets.push((path, label.into()));
}

/// Returns system and user application search paths.
pub fn app_search_paths() -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        let mut paths = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
        ];
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Applications"));
        }
        paths
    } else if cfg!(target_os = "linux") {
        let mut paths = vec![
            PathBuf::from("/usr/local/share/applications"),
            PathBuf::from("/usr/share/applications"),
        ];
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".local/share/applications"));
        }
        paths
    } else {
        vec![]
    }
}

/// Check whether a path is safe to delete. Both the selected path and the
/// allowed root are canonicalised so `..` components and symlinks cannot
/// escape the allowed directory.
pub fn is_safe_to_delete(path: &Path, allowed_prefixes: &[PathBuf]) -> bool {
    let Ok(candidate) = fs::canonicalize(path) else {
        return false;
    };

    allowed_prefixes.iter().any(|prefix| {
        let Ok(root) = fs::canonicalize(prefix) else {
            return false;
        };

        candidate != root && candidate.starts_with(root)
    })
}

/// Check whether a folder selected in the native picker is a user/content
/// location that Aether may scan and use as a deletion root for this session.
/// System locations are intentionally rejected even if a caller fabricates an
/// IPC request instead of using the picker.
pub fn is_safe_selected_scan_root(path: &Path) -> bool {
    let Ok(candidate) = fs::canonicalize(path) else {
        return false;
    };

    if is_protected_path(&candidate.to_string_lossy()) {
        return false;
    }

    let blocked = if cfg!(target_os = "macos") {
        [
            "/System",
            "/Applications",
            "/usr",
            "/bin",
            "/sbin",
            "/private",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>()
    } else if cfg!(target_os = "linux") {
        [
            "/usr", "/var", "/etc", "/bin", "/sbin", "/lib", "/lib64", "/proc", "/sys",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>()
    } else if cfg!(target_os = "windows") {
        [
            r"C:\Windows",
            r"C:\Program Files",
            r"C:\Program Files (x86)",
            r"C:\ProgramData",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    !blocked.into_iter().any(|root| {
        fs::canonicalize(root)
            .map(|root| candidate == root || candidate.starts_with(root))
            .unwrap_or(false)
    })
}

/// Returns user-owned directories where application artifacts are safe to
/// delete. `/Applications` is included on macOS so a confirmed removal of a
/// non-system application actually works; `/System/Applications` is never
/// allowed.
pub fn safe_delete_paths() -> Vec<PathBuf> {
    let home = home_dir();

    if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/Applications"),
            home.join("Applications"),
            home.join("Library").join("Application Support"),
            home.join("Library").join("Caches"),
            home.join("Library").join("Logs"),
            home.join("Library").join("Preferences"),
            home.join("Library").join("Saved Application State"),
            home.join("Library").join("Containers"),
            home.join("Library").join("Group Containers"),
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            home.join(".local").join("share"),
            home.join(".cache"),
            home.join(".config"),
            home.join(".local").join("state"),
        ]
    } else if cfg!(target_os = "windows") {
        let local = dirs::data_local_dir().unwrap_or_else(|| home.join("AppData").join("Local"));
        let roaming = dirs::config_dir().unwrap_or_else(|| home.join("AppData").join("Roaming"));
        vec![local, roaming]
    } else {
        vec![]
    }
}

/// Returns protected system paths that must never be deleted.
pub fn protected_paths() -> Vec<PathBuf> {
    let home = home_dir();
    let mut protected = vec![
        PathBuf::from("/"),
        PathBuf::from("/Applications"),
        PathBuf::from("/System"),
        PathBuf::from("/System/Applications"),
        home.clone(),
    ];

    if cfg!(target_os = "macos") {
        protected.push(home.join("Library"));
    } else if cfg!(target_os = "linux") {
        protected.extend([
            PathBuf::from("/usr"),
            PathBuf::from("/var"),
            PathBuf::from("/etc"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/lib"),
            PathBuf::from("/lib64"),
            PathBuf::from("/boot"),
            PathBuf::from("/proc"),
            PathBuf::from("/sys"),
        ]);
    } else if cfg!(target_os = "windows") {
        protected.extend([
            PathBuf::from(r"C:\Windows"),
            PathBuf::from(r"C:\Program Files"),
            PathBuf::from(r"C:\Program Files (x86)"),
            PathBuf::from(r"C:\ProgramData"),
        ]);
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            protected.push(PathBuf::from(user_profile));
        }
    }

    protected
}

/// Check if a path is explicitly protected (system root, profile root, etc.).
pub fn is_protected_path(path_str: &str) -> bool {
    let input = PathBuf::from(path_str);
    let candidate = path_identity(&input);

    protected_paths()
        .into_iter()
        .any(|protected| candidate == path_identity(&protected))
}

/// Reveal a file or directory in the system file manager / explorer.
pub fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }

    if cfg!(target_os = "macos") {
        Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open in Finder: {e}"))?;
    } else if cfg!(target_os = "linux") {
        let parent = path.parent().unwrap_or(path);
        for manager in ["xdg-open", "gio", "gnome-open", "kde-open", "exo-open"] {
            let mut command = background_command(manager);
            if manager == "gio" {
                command.arg("open");
            }
            if command.arg(parent).spawn().is_ok() {
                return Ok(());
            }
        }
        return Err(
            "No file manager found. Install xdg-utils or a desktop environment.".to_string(),
        );
    } else if cfg!(target_os = "windows") {
        let path_str = path.to_string_lossy().replace('/', "\\");
        background_command("explorer.exe")
            .args(["/select,", &path_str])
            .spawn()
            .map_err(|e| format!("Failed to open in Explorer: {e}"))?;
    } else {
        return Err("Unsupported platform for reveal_in_file_manager".to_string());
    }

    Ok(())
}

/// Get the user's home directory, falling back to a sensible default.
pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| {
        if cfg!(target_os = "windows") {
            PathBuf::from(r"C:\Users\Default")
        } else {
            PathBuf::from("/tmp")
        }
    })
}

/// Recursively compute the total file size of a directory in bytes. Individual
/// unreadable entries are skipped; a single protected child must not turn an
/// otherwise valid directory into a misleading 0 B result. Directory metadata
/// is intentionally excluded: it is not reclaimable file content and avoiding
/// those extra metadata reads keeps large scans responsive.
pub fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok())
        .map(|metadata| metadata.len())
        .sum()
}

/// Format bytes into a human-readable string (e.g. "1.5 MB").
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;

    if b < KB {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.1} MB", b / MB)
    } else if b < TB {
        format!("{:.1} GB", b / GB)
    } else {
        format!("{:.1} TB", b / TB)
    }
}
