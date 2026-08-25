use std::path::Path;
use std::fs;
use crate::models::cleaner::CleanableItem;
use uuid::Uuid;

fn get_dir_size(path: impl AsRef<Path>) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    size += get_dir_size(entry.path());
                } else {
                    size += metadata.len();
                }
            }
        }
    }
    size
}

pub fn scan_cleanable_items() -> Vec<CleanableItem> {
    let mut items = Vec::new();
    
    if let Some(home_dir) = dirs::home_dir() {
        let targets = vec![
            (home_dir.join("Library/Caches"), "Cache"),
            (home_dir.join("Library/Logs"), "Log"),
        ];

        for (target_dir, item_type) in targets {
            if !target_dir.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&target_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                        
                    // Skip hidden files
                    if name.starts_with('.') {
                        continue;
                    }

                    let size_bytes = if path.is_dir() {
                        get_dir_size(&path)
                    } else {
                        entry.metadata().map(|m| m.len()).unwrap_or(0)
                    };
                    
                    // Only include items larger than 1MB to avoid cluttering the UI
                    // or maybe just include everything but sort it. Let's include items > 100KB.
                    if size_bytes > 100 * 1024 {
                        items.push(CleanableItem {
                            id: Uuid::new_v4().to_string(),
                            name,
                            absolute_path: path.to_string_lossy().to_string(),
                            size_bytes,
                            item_type: item_type.to_string(),
                        });
                    }
                }
            }
        }
    }
    
    items
}
