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

pub fn scan_application_artifacts(bundle_id: Option<&str>, app_name: &str) -> Vec<Artifact> {
    let mut artifacts = Vec::new();
    let home = match dirs::home_dir() {
        Some(dir) => dir,
        None => return artifacts,
    };
    
    // Candidates structure: (Path, Category, Confidence)
    let mut candidates: Vec<(PathBuf, &str, ArtifactConfidence)> = Vec::new();
    
    // 1. App Support
    candidates.push((home.join("Library/Application Support").join(app_name), "Application Support", ArtifactConfidence::High));
    if let Some(bid) = bundle_id {
        candidates.push((home.join("Library/Application Support").join(bid), "Application Support", ArtifactConfidence::Exact));
    }
    
    // 2. Caches
    candidates.push((home.join("Library/Caches").join(app_name), "Caches", ArtifactConfidence::High));
    if let Some(bid) = bundle_id {
        candidates.push((home.join("Library/Caches").join(bid), "Caches", ArtifactConfidence::Exact));
    }
    
    // 3. Preferences
    if let Some(bid) = bundle_id {
        candidates.push((home.join("Library/Preferences").join(format!("{}.plist", bid)), "Preferences", ArtifactConfidence::Exact));
    }
    
    // 4. Saved Application State
    if let Some(bid) = bundle_id {
        candidates.push((home.join("Library/Saved Application State").join(format!("{}.savedState", bid)), "Saved State", ArtifactConfidence::Exact));
    }
    
    // 5. Containers
    if let Some(bid) = bundle_id {
        candidates.push((home.join("Library/Containers").join(bid), "Containers", ArtifactConfidence::Exact));
    }
    
    // 6. Logs
    candidates.push((home.join("Library/Logs").join(app_name), "Logs", ArtifactConfidence::High));
    if let Some(bid) = bundle_id {
        candidates.push((home.join("Library/Logs").join(bid), "Logs", ArtifactConfidence::Exact));
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
