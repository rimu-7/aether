use std::path::{Path, PathBuf};
use crate::models::artifact::{Artifact, ArtifactConfidence};

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

pub fn scan_application_artifacts(bundle_id: Option<&str>, app_name: &str, bundle_path: &str) -> Vec<Artifact> {
    let mut artifacts = Vec::new();
    let home = match dirs::home_dir() {
        Some(dir) => dir,
        None => return artifacts,
    };

    let mut candidates: Vec<(PathBuf, &str, ArtifactConfidence)> = Vec::new();

    if cfg!(target_os = "macos") {
        // 1. App Support
        candidates.push((home.join("Library/Application Support").join(app_name), "Application Support", ArtifactConfidence::High));
        if let Some(bid) = bundle_id {
            candidates.push((home.join("Library/Application Support").join(bid), "Application Support", ArtifactConfidence::Exact));
        }

        // 2. Caches
        candidates.push((home.join("Library/Caches").join(app_name), "Cache", ArtifactConfidence::High));
        if let Some(bid) = bundle_id {
            candidates.push((home.join("Library/Caches").join(bid), "Cache", ArtifactConfidence::Exact));
        }

        // 3. Preferences
        if let Some(bid) = bundle_id {
            candidates.push((home.join("Library/Preferences").join(format!("{}.plist", bid)), "Preference", ArtifactConfidence::Exact));
        }

        // 4. Saved Application State
        if let Some(bid) = bundle_id {
            candidates.push((home.join("Library/Saved Application State").join(format!("{}.savedState", bid)), "Saved State", ArtifactConfidence::Exact));
        }

        // 5. Containers
        if let Some(bid) = bundle_id {
            candidates.push((home.join("Library/Containers").join(bid), "Container", ArtifactConfidence::Exact));
        }

        // 6. Logs
        candidates.push((home.join("Library/Logs").join(app_name), "Log", ArtifactConfidence::High));
        if let Some(bid) = bundle_id {
            candidates.push((home.join("Library/Logs").join(bid), "Log", ArtifactConfidence::Exact));
        }
    } else if cfg!(target_os = "linux") {
        // XDG Base Directory spec
        let config_home = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"));

        // 1. Config (Application Support equivalent)
        candidates.push((config_home.join(app_name.to_lowercase()), "Config", ArtifactConfidence::High));
        if let Some(bid) = bundle_id {
            candidates.push((config_home.join(bid), "Config", ArtifactConfidence::High));
        }

        // 2. Caches
        candidates.push((home.join(".cache").join(app_name.to_lowercase()), "Cache", ArtifactConfidence::High));
        if let Some(bid) = bundle_id {
            candidates.push((home.join(".cache").join(bid), "Cache", ArtifactConfidence::High));
        }

        // 3. Config files (preferences equivalent)
        if let Some(bid) = bundle_id {
            candidates.push((config_home.join(format!("{}.conf", bid)), "Preference", ArtifactConfidence::Medium));
        }
        candidates.push((config_home.join(format!("{}.conf", app_name.to_lowercase())), "Preference", ArtifactConfidence::Medium));

        // 4. Logs
        let local_share = home.join(".local/share");
        candidates.push((local_share.join("Trash").join(app_name.to_lowercase()), "Trash", ArtifactConfidence::Medium));
        candidates.push((home.join(".local/state").join(app_name.to_lowercase()), "State", ArtifactConfidence::Medium));
    } else if cfg!(target_os = "windows") {
        let local_app_data = home.join("AppData").join("Local");
        let app_data = home.join("AppData").join("Roaming");

        // 1. Local App Data (Application Support equivalent)
        candidates.push((local_app_data.join(app_name), "Application Data", ArtifactConfidence::High));
        if let Some(bid) = bundle_id {
            candidates.push((local_app_data.join(bid), "Application Data", ArtifactConfidence::Exact));
        }

        // 2. Caches
        candidates.push((local_app_data.join("Temp").join(app_name), "Cache", ArtifactConfidence::High));
        if let Some(bid) = bundle_id {
            candidates.push((local_app_data.join("Temp").join(bid), "Cache", ArtifactConfidence::High));
        }

        // 3. AppData Roaming (preferences equivalent)
        candidates.push((app_data.join(app_name), "Roaming", ArtifactConfidence::High));
        if let Some(bid) = bundle_id {
            candidates.push((app_data.join(bid), "Roaming", ArtifactConfidence::Exact));
        }

        // 4. Temp
        if let Some(bid) = bundle_id {
            candidates.push((local_app_data.join("Temp").join(bid), "Temp", ArtifactConfidence::Medium));
        }
    }

    // Also check for artifacts near the app bundle itself
    let bundle = Path::new(bundle_path);
    if bundle.exists() {
        if let Some(parent) = bundle.parent() {
            // Look for sibling config/data directories
            let parent_name = bundle.file_name().and_then(|n| n.to_str()).unwrap_or("");
            candidates.push((parent.join(format!("{}.config", parent_name)), "Nearby Config", ArtifactConfidence::Low));
        }
    }

    for (path, category, confidence) in candidates {
        if path.exists() {
            let size_bytes = if path.is_file() {
                std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            } else {
                get_dir_size(&path)
            };

            artifacts.push(Artifact {
                path: path.to_string_lossy().to_string(),
                category: category.to_string(),
                confidence,
                size_bytes,
            });
        }
    }

    artifacts
}
