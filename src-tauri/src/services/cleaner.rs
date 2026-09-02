use std::fs;
use crate::models::cleaner::CleanableItem;
use crate::platform::utils::{cleanable_targets, dir_size};
use uuid::Uuid;

pub fn scan_cleanable_items() -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let targets = cleanable_targets();

    for (target_dir, item_type) in &targets {
        if !target_dir.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(target_dir) {
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
                    dir_size(&path)
                } else {
                    entry.metadata().map(|m| m.len()).unwrap_or(0)
                };

                // Only include items larger than 100KB to avoid cluttering the UI
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

    // Sort by size descending for better UX
    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    items
}
