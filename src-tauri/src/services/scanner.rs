use crate::models::application::Application;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use crate::platform::utils::{app_search_paths, dir_size};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::fs;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use crate::platform::windows_registry;

pub fn scan_applications() -> Vec<Application> {
    #[cfg(target_os = "macos")]
    {
        scan_applications_macos()
    }
    #[cfg(target_os = "linux")]
    {
        scan_applications_linux()
    }
    #[cfg(target_os = "windows")]
    {
        scan_applications_windows()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        vec![]
    }
}

#[cfg(target_os = "macos")]
fn scan_applications_macos() -> Vec<Application> {
    let search_paths = app_search_paths();
    let mut app_paths = Vec::new();

    for base_dir in search_paths {
        if !base_dir.exists() {
            continue;
        }

        let is_system_dir = base_dir.starts_with("/System");

        if let Ok(entries) = fs::read_dir(&base_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() && path.extension().and_then(|s| s.to_str()) == Some("app") {
                    let info_plist = path.join("Contents/Info.plist");
                    if info_plist.exists() {
                        app_paths.push((path, is_system_dir, info_plist));
                    }
                }
            }
        }
    }

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(16);

    let chunk_size = (app_paths.len() + num_threads - 1) / num_threads;
    let chunks: Vec<_> = app_paths.chunks(chunk_size.max(1)).collect();

    let mut results = Vec::new();
    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for chunk in chunks {
            handles.push(s.spawn(move || {
                let mut chunk_apps = Vec::new();
                for (path, is_system_dir, info_plist) in chunk {
                    if let Ok(dict) = plist::Value::from_file(info_plist) {
                        if let Some(dict) = dict.as_dictionary() {
                            let bundle_id = dict
                                .get("CFBundleIdentifier")
                                .and_then(|v| v.as_string())
                                .map(String::from);
                            let name = dict
                                .get("CFBundleName")
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| {
                                    path.file_stem().unwrap().to_string_lossy().to_string()
                                });

                            let display_name = dict
                                .get("CFBundleDisplayName")
                                .and_then(|v| v.as_string())
                                .unwrap_or(&name)
                                .to_string();
                            let version = dict
                                .get("CFBundleShortVersionString")
                                .and_then(|v| v.as_string())
                                .map(String::from);
                            let executable = dict
                                .get("CFBundleExecutable")
                                .and_then(|v| v.as_string())
                                .map(String::from);

                            let size_bytes = dir_size(path);

                            chunk_apps.push(Application {
                                id: bundle_id
                                    .clone()
                                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                                bundle_id,
                                name,
                                display_name,
                                version,
                                developer: None,
                                bundle_path: path.to_string_lossy().to_string(),
                                executable_path: executable.map(|e| {
                                    path.join("Contents/MacOS")
                                        .join(e)
                                        .to_string_lossy()
                                        .to_string()
                                }),
                                icon_path: None,
                                is_system: *is_system_dir,
                                is_running: false,
                                size_bytes,
                            });
                        }
                    }
                }
                chunk_apps
            }));
        }

        for handle in handles {
            if let Ok(apps) = handle.join() {
                results.extend(apps);
            }
        }
    });

    results
}

#[cfg(target_os = "linux")]
fn scan_applications_linux() -> Vec<Application> {
    use std::collections::HashSet;

    let mut apps = Vec::new();
    let mut search_paths = app_search_paths();
    if let Some(home) = dirs::home_dir() {
        search_paths.extend([
            home.join(".local/share/flatpak/exports/share/applications"),
            home.join(".local/share/nix-profile/share/applications"),
        ]);
    }
    search_paths.extend([
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
        PathBuf::from("/run/current-system/sw/share/applications"),
    ]);

    if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
        for data_dir in std::env::split_paths(&data_dirs) {
            search_paths.push(data_dir.join("applications"));
        }
    }

    let mut seen_paths = HashSet::new();
    for base_dir in search_paths {
        let identity = std::fs::canonicalize(&base_dir).unwrap_or_else(|_| base_dir.clone());
        if !base_dir.is_dir() || !seen_paths.insert(identity) {
            continue;
        }

        let is_system_dir = !base_dir.starts_with(dirs::home_dir().unwrap_or_default());

        if let Ok(entries) = fs::read_dir(&base_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut name = String::new();
                        let mut exec = String::new();
                        let mut icon = String::new();
                        let mut version = String::new();
                        let mut is_app = false;

                        for line in content.lines() {
                            if line.starts_with("Name=") && name.is_empty() {
                                name = line.trim_start_matches("Name=").trim().to_string();
                            } else if line.starts_with("Exec=") {
                                exec = line.trim_start_matches("Exec=").trim().to_string();
                            } else if line.starts_with("Icon=") {
                                icon = line.trim_start_matches("Icon=").trim().to_string();
                                if let Some(dot_idx) = icon.rfind('.') {
                                    icon = icon[..dot_idx].to_string();
                                }
                            } else if line.starts_with("Version=") {
                                version = line.trim_start_matches("Version=").trim().to_string();
                            } else if line.starts_with("Type=") {
                                let val = line.trim_start_matches("Type=").trim();
                                is_app = val == "Application";
                            }
                        }

                        if is_app && !name.is_empty() {
                            let icon_path = resolve_linux_icon_path(&icon);
                            let executable_path = resolve_linux_executable(&exec);
                            let launcher_size =
                                entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
                            let executable_size = executable_path
                                .as_ref()
                                .and_then(|executable| std::fs::metadata(executable).ok())
                                .map(|metadata| metadata.len())
                                .unwrap_or(0);

                            apps.push(Application {
                                id: path.to_string_lossy().to_string(),
                                bundle_id: None,
                                name: name.clone(),
                                display_name: name,
                                version: if version.is_empty() {
                                    None
                                } else {
                                    Some(version)
                                },
                                developer: None,
                                bundle_path: path.to_string_lossy().to_string(),
                                executable_path: executable_path
                                    .map(|path| path.to_string_lossy().to_string())
                                    .or_else(|| (!exec.is_empty()).then_some(exec)),
                                icon_path,
                                is_system: is_system_dir,
                                is_running: false,
                                // A .desktop launcher does not store an install
                                // directory. Use the actual launcher/executable
                                // size rather than falsely reporting 0 B; the
                                // package view exposes full package sizes where
                                // the system package manager provides them.
                                size_bytes: launcher_size.saturating_add(executable_size),
                            });
                        }
                    }
                }
            }
        }
    }

    apps
}

/// Resolve the executable token in a freedesktop `Exec=` field. Field codes
/// (`%U`, `%f`, …) and environment assignments are ignored for sizing only.
#[cfg(target_os = "linux")]
fn resolve_linux_executable(exec: &str) -> Option<PathBuf> {
    let mut tokens = exec.split_whitespace();
    let mut executable = tokens.next()?;

    if executable == "env" {
        executable = tokens.find(|token| !token.contains('='))?;
    }
    while executable.contains('=') {
        executable = tokens.next()?;
    }

    let executable = executable.trim_matches(['\"', '\'']);
    if executable.starts_with('%') || executable.is_empty() {
        return None;
    }

    let path = PathBuf::from(executable);
    if path.is_absolute() {
        return path.is_file().then_some(path);
    }

    let mut search_paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    search_paths.extend([
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ]);
    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join(".local/bin"));
    }

    search_paths
        .into_iter()
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
}

/// Resolve a Linux icon name to an actual file path by searching standard theme locations.
#[cfg(target_os = "linux")]
fn resolve_linux_icon_path(icon_name: &str) -> Option<String> {
    if icon_name.is_empty() {
        return None;
    }

    // If it's an absolute path, use it directly
    let icon_path = PathBuf::from(icon_name);
    if icon_path.is_absolute() {
        if icon_path.exists() {
            return Some(icon_path.to_string_lossy().to_string());
        }
        // Try with common extensions
        for ext in &["png", "svg", "xpm"] {
            let candidate = PathBuf::from(format!("{}.{}", icon_name, ext));
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
        return None;
    }

    let home = dirs::home_dir()?;
    let search_roots = [
        home.join(".local/share/icons"),
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
    ];

    let sizes = [
        "scalable", "256x256", "128x128", "64x64", "48x48", "32x32", "24x24", "16x16",
    ];
    let extensions = ["png", "svg", "xpm"];

    for root in &search_roots {
        // Try hicolor structure: root/{size}/apps/{name}.{ext}
        for size in &sizes {
            let apps_dir = root.join(size).join("apps");
            for ext in &extensions {
                let candidate = apps_dir.join(format!("{}.{}", icon_name, ext));
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
        }

        // Try root-level (e.g., /usr/share/pixmaps/{name}.{ext})
        for ext in &extensions {
            let candidate = root.join(format!("{}.{}", icon_name, ext));
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn scan_applications_windows() -> Vec<Application> {
    windows_registry::installed_programs()
        .into_iter()
        .filter(|program| !program.is_system_component)
        .map(|program| {
            let executable_path = program.display_icon.as_ref().and_then(|icon| {
                let value = icon.trim().trim_matches('\"');
                let value = value.split(',').next().unwrap_or(value).trim_matches('\"');
                (!value.is_empty()).then(|| value.to_string())
            });

            let bundle_path = program
                .install_location
                .clone()
                .filter(|path| !path.trim().is_empty())
                .or_else(|| {
                    executable_path.as_ref().and_then(|path| {
                        Path::new(path)
                            .parent()
                            .map(|parent| parent.to_string_lossy().to_string())
                    })
                })
                // Some registry entries have neither value. Keep their
                // registry ID as a non-deletable display location instead of
                // silently dropping a real installed application.
                .unwrap_or_else(|| "Registry-managed application".to_string());

            let fallback_size = executable_path
                .as_deref()
                .and_then(|path| std::fs::metadata(path).ok())
                .map(|metadata| metadata.len())
                .unwrap_or(0);

            Application {
                id: program.id,
                bundle_id: None,
                name: program.display_name.clone(),
                display_name: program.display_name,
                version: program.display_version,
                developer: program.publisher,
                bundle_path,
                executable_path,
                icon_path: program.display_icon,
                is_system: false,
                is_running: false,
                // EstimatedSize is supplied by the installer in KiB. If it is
                // absent, use the real executable size instead of blocking the
                // whole inventory by recursively walking every install folder.
                size_bytes: program.estimated_size_bytes.unwrap_or(fallback_size),
            }
        })
        .collect()
}
