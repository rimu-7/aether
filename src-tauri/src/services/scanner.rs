use std::path::{Path, PathBuf};
use std::fs;
use crate::models::application::Application;

fn get_dir_size(path: &Path) -> u64 {
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
    let mut apps = Vec::new();
    let mut search_paths = Vec::new();
    search_paths.push(PathBuf::from("/Applications"));
    search_paths.push(PathBuf::from("/System/Applications"));
    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join("Applications"));
    }

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
                        if let Ok(dict) = plist::Value::from_file(&info_plist) {
                            if let Some(dict) = dict.as_dictionary() {
                                let bundle_id = dict.get("CFBundleIdentifier").and_then(|v| v.as_string()).map(String::from);
                                let name = dict.get("CFBundleName")
                                    .and_then(|v| v.as_string())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| {
                                        path.file_stem().unwrap().to_string_lossy().to_string()
                                    });

                                let display_name = dict.get("CFBundleDisplayName")
                                    .and_then(|v| v.as_string())
                                    .unwrap_or(&name)
                                    .to_string();
                                let version = dict.get("CFBundleShortVersionString")
                                    .and_then(|v| v.as_string())
                                    .map(String::from);
                                let executable = dict.get("CFBundleExecutable")
                                    .and_then(|v| v.as_string())
                                    .map(String::from);

                                let size_bytes = get_dir_size(&path);

                                apps.push(Application {
                                    id: bundle_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                                    bundle_id,
                                    name,
                                    display_name,
                                    version,
                                    developer: None,
                                    bundle_path: path.to_string_lossy().to_string(),
                                    executable_path: executable.map(|e| path.join("Contents/MacOS").join(e).to_string_lossy().to_string()),
                                    icon_path: None,
                                    is_system: is_system_dir,
                                    is_running: false,
                                    size_bytes,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    apps
}

#[cfg(target_os = "linux")]
fn scan_applications_linux() -> Vec<Application> {
    let mut apps = Vec::new();
    let mut search_paths = Vec::new();
    search_paths.push(PathBuf::from("/usr/share/applications"));
    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join(".local/share/applications"));
    }

    for base_dir in search_paths {
        if !base_dir.exists() {
            continue;
        }

        let is_system_dir = base_dir.starts_with("/usr");

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
                                exec = line.trim_start_matches("Exec=").split_whitespace().next().unwrap_or("").to_string();
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

                            apps.push(Application {
                                id: uuid::Uuid::new_v4().to_string(),
                                bundle_id: None,
                                name: name.clone(),
                                display_name: name,
                                version: if version.is_empty() { None } else { Some(version) },
                                developer: None,
                                bundle_path: path.to_string_lossy().to_string(),
                                executable_path: if exec.is_empty() { None } else { Some(exec) },
                                icon_path,
                                is_system: is_system_dir,
                                is_running: false,
                                size_bytes: 0,
                            });
                        }
                    }
                }
            }
        }
    }

    apps
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

    let sizes = ["scalable", "256x256", "128x128", "64x64", "48x48", "32x32", "24x24", "16x16"];
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
    let mut apps = Vec::new();

    // Query the Windows registry for installed applications
    let ps_script = r#"
        Get-ItemProperty `
            HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*, `
            HKLM:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*, `
            "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" `
            -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -ne $null } |
        Select-Object DisplayName, DisplayVersion, Publisher, InstallLocation, DisplayIcon |
        ConvertTo-Json -Compress
    "#;

    if let Ok(output) = std::process::Command::new("powershell")
        .args(&["-NoProfile", "-Command", ps_script])
        .output()
    {
        if let Ok(json_str) = String::from_utf8(output.stdout) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(arr) = parsed.as_array() {
                    for item in arr {
                        let name = item.get("DisplayName")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        if name.is_none() || name.as_ref().unwrap().trim().is_empty() {
                            continue;
                        }

                        let name = name.unwrap();
                        let version = item.get("DisplayVersion")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let developer = item.get("Publisher")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        // InstallLocation might be empty or None
                        let install_location = item.get("InstallLocation")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(String::from);

                        // DisplayIcon often points to the exe with icon index, e.g. "C:\App\app.exe,0"
                        let display_icon = item.get("DisplayIcon")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(String::from);

                        // Determine bundle_path: use InstallLocation or derive from DisplayIcon
                        let bundle_path = install_location.clone()
                            .or_else(|| {
                                display_icon.as_ref().and_then(|icon| {
                                    // DisplayIcon format: "path.exe,0" or "path.exe"
                                    let path_part = icon.split(',').next()?.trim();
                                    let exe_path = Path::new(path_part);
                                    exe_path.parent().map(|p| p.to_string_lossy().to_string())
                                })
                            })
                            .unwrap_or_else(|| "Unknown".to_string());

                        // Determine executable path from DisplayIcon
                        let executable_path = display_icon.as_ref()
                            .and_then(|icon| {
                                let path_part = icon.split(',').next()?.trim();
                                Some(path_part.to_string())
                            });

                        // Skip system components that have no meaningful install location
                        if bundle_path == "Unknown" && executable_path.is_none() {
                            continue;
                        }

                        // is_system indicates whether this is a system component (no DisplayIcon) vs a user app
                        let is_system = display_icon.is_none();

                        apps.push(Application {
                            id: uuid::Uuid::new_v4().to_string(),
                            bundle_id: None,
                            name: name.clone(),
                            display_name: name,
                            version,
                            developer,
                            bundle_path,
                            executable_path,
                            icon_path: display_icon,
                            is_system,
                            is_running: false,
                            size_bytes: 0,
                        });
                    }
                }
            }
        }
    }

    apps
}
