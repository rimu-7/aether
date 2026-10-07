use crate::models::cleaner::CleanableItem;
use crate::platform::utils::{cleanable_targets, dir_size};
use std::fs;
use uuid::Uuid;

pub fn scan_cleanable_items() -> Vec<CleanableItem> {
    let targets = cleanable_targets();
    let mut raw_entries = Vec::new();

    for (target_dir, item_type) in &targets {
        if !target_dir.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let shallow_len = if !is_dir {
                    entry.metadata().map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };

                raw_entries.push((path, name, item_type.clone(), is_dir, shallow_len));
            }
        }
    }

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(16);

    let chunk_size = (raw_entries.len() + num_threads - 1) / num_threads;
    let chunks: Vec<_> = raw_entries.chunks(chunk_size.max(1)).collect();

    let mut items = Vec::new();
    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for chunk in chunks {
            handles.push(s.spawn(move || {
                let mut chunk_items = Vec::new();
                for (path, name, item_type, is_dir, shallow_len) in chunk {
                    let size_bytes = if *is_dir {
                        dir_size(path)
                    } else {
                        *shallow_len
                    };

                    chunk_items.push(CleanableItem {
                        id: Uuid::new_v4().to_string(),
                        name: name.clone(),
                        absolute_path: path.to_string_lossy().to_string(),
                        size_bytes,
                        item_type: item_type.clone(),
                    });
                }
                chunk_items
            }));
        }

        for handle in handles {
            if let Ok(chunk_items) = handle.join() {
                items.extend(chunk_items);
            }
        }
    });

    // Sort by size descending for better UX
    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    items
}
