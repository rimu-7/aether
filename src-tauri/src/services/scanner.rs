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
    let mut search_paths = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];

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
                }
            }
        }
    }

    apps
}
