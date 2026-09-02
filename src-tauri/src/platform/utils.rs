use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns user-owned directories commonly used for cache, logs, and temp files.
/// Each entry is a (directory_path, item_type_label) tuple used by the cleaner.
pub fn cleanable_targets() -> Vec<(PathBuf, &'static str)> {
    if cfg!(target_os = "macos") {
        let home = home_dir();
        vec![
            (home.join("Library/Caches"), "Cache"),
            (home.join("Library/Logs"), "Log"),
            (home.join("Library/Application Support"), "Application Support"),
        ]
    } else if cfg!(target_os = "linux") {
        let home = home_dir();
        vec![
            (home.join(".cache"), "Cache"),
            (home.join(".local/share/Trash"), "Trash"),
        ]
    } else if cfg!(target_os = "windows") {
        let home = home_dir();
        vec![
            (home.join("AppData\\Local\\Temp"), "Temp"),
            (home.join("AppData\\Local\\Microsoft\\Windows\\INetCache"), "Cache"),
        ]
    } else {
        vec![]
    }
}

/// Returns user-owned directories that the large-file scanner should search.
/// On Linux/Windows these mirror the macOS Downloads/Documents/Desktop/Movies structure.
pub fn user_scannable_dirs() -> Vec<(PathBuf, &'static str)> {
    let home = home_dir();
    if cfg!(target_os = "macos") {
        vec![
            (home.join("Downloads"), "Downloads"),
            (home.join("Documents"), "Documents"),
            (home.join("Desktop"), "Desktop"),
            (home.join("Movies"), "Movies"),
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            (home.join("Downloads"), "Downloads"),
            (home.join("Documents"), "Documents"),
            (home.join("Desktop"), "Desktop"),
            (home.join("Videos"), "Movies"),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            (home.join("Downloads"), "Downloads"),
            (home.join("Documents"), "Documents"),
            (home.join("Desktop"), "Desktop"),
            (home.join("Videos"), "Movies"),
        ]
    } else {
        vec![]
    }
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
            PathBuf::from("/usr/share/applications"),
        ];
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".local/share/applications"));
        }
        paths
    } else if cfg!(target_os = "windows") {
        vec![]
    } else {
        vec![]
    }
}

/// Check whether a path is safe to delete.
/// A path is safe if it is inside one of the `allowed_prefixes` but is not one of those prefixes themselves.
pub fn is_safe_to_delete(path: &Path, allowed_prefixes: &[PathBuf]) -> bool {
    for prefix in allowed_prefixes {
        if path.starts_with(prefix) && path != *prefix {
            return true;
        }
    }
    false
}

/// Returns user-owned directories where application artifacts are safe to delete.
pub fn safe_delete_paths() -> Vec<PathBuf> {
    let home = home_dir();

    if cfg!(target_os = "macos") {
        vec![
            home.join("Library/Application Support"),
            home.join("Library/Caches"),
            home.join("Library/Logs"),
            home.join("Library/Preferences"),
            home.join("Library/Saved Application State"),
            home.join("Library/Containers"),
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            home.join(".local/share"),
            home.join(".cache"),
            home.join(".config"),
            home.join(".local/state"),
            home.join(".local/share/applications"),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            home.join("AppData\\Local\\Temp"),
            home.join("AppData\\Local\\Microsoft\\Windows\\INetCache"),
            home.join("AppData\\Local"),
            home.join("AppData\\Roaming"),
        ]
    } else {
        vec![home]
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
        protected.push(PathBuf::from("/usr"));
        protected.push(PathBuf::from("/var"));
        protected.push(PathBuf::from("/etc"));
        protected.push(PathBuf::from("/bin"));
        protected.push(PathBuf::from("/sbin"));
        protected.push(PathBuf::from("/lib"));
        protected.push(PathBuf::from("/lib64"));
        protected.push(PathBuf::from("/boot"));
        protected.push(PathBuf::from("/proc"));
        protected.push(PathBuf::from("/sys"));
    } else if cfg!(target_os = "windows") {
        protected.push(PathBuf::from("C:\\Windows"));
        protected.push(PathBuf::from("C:\\Program Files"));
        protected.push(PathBuf::from("C:\\Program Files (x86)"));
        protected.push(PathBuf::from("C:\\ProgramData"));
        // User profile root itself
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            protected.push(PathBuf::from(user_profile));
        }
    }

    protected
}

/// Check if a path is explicitly protected (system root, home dir, etc.)
pub fn is_protected_path(path_str: &str) -> bool {
    let path = PathBuf::from(path_str);
    for protected in protected_paths() {
        if path == protected {
            return true;
        }
    }
    false
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
            .map_err(|e| format!("Failed to open in Finder: {}", e))?;
    } else if cfg!(target_os = "linux") {
        // Try common file managers; xdg-open opens the parent directory as a fallback
        let parent = path.parent().unwrap_or(path);
        let managers = ["xdg-open", "gnome-open", "kde-open", "exo-open"];
        for mgr in &managers {
            if let Ok(output) = Command::new(mgr).arg(parent).spawn() {
                let _ = output;
                return Ok(());
            }
        }
        // If no file manager is available, try just opening the parent directory
        if let Some(parent_str) = parent.to_str() {
            for mgr in &managers {
                if let Ok(output) = Command::new(mgr).arg(parent_str).spawn() {
                    let _ = output;
                    return Ok(());
                }
            }
        }
        return Err("No file manager found. Install xdg-utils or a desktop environment.".to_string());
    } else if cfg!(target_os = "windows") {
        // explorer /select, "C:\path\to\file" selects the file in Explorer
        let path_str = path.to_string_lossy().replace("/", "\\");
        Command::new("explorer.exe")
            .args(&["/select,", &path_str])
            .spawn()
            .map_err(|e| format!("Failed to open in Explorer: {}", e))?;
    } else {
        return Err(format!("Unsupported platform for reveal_in_file_manager"));
    }

    Ok(())
}

/// Get the user's home directory, falling back to a sensible default.
pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| {
        if cfg!(target_os = "windows") {
            PathBuf::from("C:\\Users\\Default")
        } else {
            PathBuf::from("/tmp")
        }
    })
}

/// Recursively compute the total size of a directory in bytes.
pub fn dir_size(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = walkdir::WalkDir::new(path).into_iter().collect::<Result<Vec<_>, _>>() {
        for entry in entries {
            if let Ok(metadata) = entry.metadata() {
                size += metadata.len();
            }
        }
    }
    size
}

/// Format bytes into a human-readable string (e.g. "1.5 MB").
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;

    if b < KB {
        format!("{} B", bytes)
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
