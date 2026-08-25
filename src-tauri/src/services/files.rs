use walkdir::WalkDir;
use std::time::SystemTime;
use uuid::Uuid;
use crate::models::file_item::FileItem;

pub fn scan_large_files() -> Vec<FileItem> {
    let mut items = Vec::new();
    
    if let Some(home_dir) = dirs::home_dir() {
        let targets = vec![
            (home_dir.join("Downloads"), "Downloads"),
            (home_dir.join("Documents"), "Documents"),
            (home_dir.join("Desktop"), "Desktop"),
            (home_dir.join("Movies"), "Movies"),
        ];

        // 10 MB threshold for "large" files
        let size_threshold = 10 * 1024 * 1024;

        for (target_dir, category) in targets {
            if !target_dir.exists() {
                continue;
            }

            for entry in WalkDir::new(&target_dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                
                if path.is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        let size_bytes = metadata.len();
                        
                        if size_bytes >= size_threshold {
                            let name = entry.file_name().to_string_lossy().to_string();
                            
                            // Skip hidden files
                            if name.starts_with('.') {
                                continue;
                            }
                            
                            let extension = path.extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_string();
                                
                            let last_modified = metadata.modified()
                                .ok()
                                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                                
                            items.push(FileItem {
                                id: Uuid::new_v4().to_string(),
                                name,
                                absolute_path: path.to_string_lossy().to_string(),
                                size_bytes,
                                last_modified,
                                extension,
                                category: category.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    
    items
}
