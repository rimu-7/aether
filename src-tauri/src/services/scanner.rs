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
    let mut apps = Vec::new();
    let mut search_paths = Vec::new();
    if cfg!(target_os = "macos") {
        search_paths.push(PathBuf::from("/Applications"));
        search_paths.push(PathBuf::from("/System/Applications"));
        if let Some(home) = dirs::home_dir() {
            search_paths.push(home.join("Applications"));
        }
    } else if cfg!(target_os = "linux") {
        search_paths.push(PathBuf::from("/usr/share/applications"));
        if let Some(home) = dirs::home_dir() {
            search_paths.push(home.join(".local/share/applications"));
        }
    } else if cfg!(target_os = "windows") {
        // Windows applications are best queried via Registry instead of scanning start menu .lnk files without a crate
        use std::process::Command;
        if let Ok(output) = Command::new("powershell")
            .args(&["-NoProfile", "-Command", r#"
                Get-ItemProperty HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*, HKLM:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall\* -ErrorAction SilentlyContinue | 
                Where-Object { $_.DisplayName -ne $null } | 
                Select-Object DisplayName, DisplayVersion, Publisher, InstallLocation | 
                ConvertTo-Json -Compress
            "#])
            .output() {
            
            if let Ok(json_str) = String::from_utf8(output.stdout) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(arr) = parsed.as_array() {
                        for item in arr {
                            if let Some(name) = item.get("DisplayName").and_then(|v| v.as_str()) {
                                let version = item.get("DisplayVersion").and_then(|v| v.as_str()).map(String::from);
                                let developer = item.get("Publisher").and_then(|v| v.as_str()).map(String::from);
                                let install_location = item.get("InstallLocation").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                                
                                apps.push(Application {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    bundle_id: None,
                                    name: name.to_string(),
                                    display_name: name.to_string(),
                                    version,
                                    developer,
                                    bundle_path: install_location.clone(),
                                    executable_path: None,
                                    icon_path: None,
                                    is_system: false,
                                    is_running: false,
                                    size_bytes: 0,
                                });
                            }
                        }
                    }
                }
            }
        }
        return apps;
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
                                
                                let display_name = dict.get("CFBundleDisplayName").and_then(|v| v.as_string()).unwrap_or(&name).to_string();
                                let version = dict.get("CFBundleShortVersionString").and_then(|v| v.as_string()).map(String::from);
                                let executable = dict.get("CFBundleExecutable").and_then(|v| v.as_string()).map(String::from);
                                
                                let size_bytes = get_dir_size(&path);

                                apps.push(Application {
                                    id: bundle_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                                    bundle_id,
                                    name,
                                    display_name,
                                    version,
                                    developer: None, // Hard to extract without code signature
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
                } else if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    // Linux desktop files
                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut name = String::new();
                        let mut exec = String::new();
                        let mut is_app = false;
                        
                        for line in content.lines() {
                            if line.starts_with("Name=") && name.is_empty() {
                                name = line.trim_start_matches("Name=").to_string();
                            } else if line.starts_with("Exec=") {
                                exec = line.trim_start_matches("Exec=").split_whitespace().next().unwrap_or("").to_string();
                            } else if line.starts_with("Type=Application") {
                                is_app = true;
                            }
                        }
                        
                        if is_app && !name.is_empty() {
                            apps.push(Application {
                                id: uuid::Uuid::new_v4().to_string(),
                                bundle_id: None,
                                name: name.clone(),
                                display_name: name,
                                version: None,
                                developer: None,
                                bundle_path: path.to_string_lossy().to_string(),
                                executable_path: if exec.is_empty() { None } else { Some(exec) },
                                icon_path: None,
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
