use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

use uuid::Uuid;
use walkdir::WalkDir;

use crate::models::file_item::FileItem;
use crate::platform::utils::user_scannable_dirs;

pub fn scan_large_files(additional_roots: &[PathBuf]) -> Vec<FileItem> {
    let mut items = Vec::new();

    let mut targets = user_scannable_dirs();
    let mut known_roots: HashSet<PathBuf> = targets
        .iter()
        .map(|(path, _)| std::fs::canonicalize(path).unwrap_or_else(|_| path.clone()))
        .collect();

    for root in additional_roots {
        if !root.is_dir() {
            continue;
        }
        let identity = std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
        // A chosen folder that is already covered by a default visible user
        // directory would otherwise be scanned twice, making a large scan
        // noticeably slower and returning duplicate rows.
        if known_roots
            .iter()
            .any(|known_root| identity.starts_with(known_root))
        {
            continue;
        }
        known_roots.insert(identity);
        let category = root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(|name| format!("Selected: {name}"))
            .unwrap_or_else(|| "Selected folder".to_string());
        targets.push((root.clone(), category));
    }

    // A 1 MB threshold keeps this view focused on meaningful files while no
    // longer making normal user folders look empty on a fresh Linux/Windows
    // installation. Cache and temporary files are exposed separately by the
    // cleaner.
    let size_threshold = 1024 * 1024;

    for (target_dir, category) in &targets {
        if !target_dir.exists() {
            continue;
        }

        let walker = WalkDir::new(target_dir)
            .follow_links(false)
            // Do not unexpectedly recurse into a mounted filesystem beneath
            // a user folder. A user can explicitly choose that mount when it
            // is relevant.
            .same_file_system(true)
            .min_depth(1)
            .into_iter()
            // Hidden entries were already excluded from results. Pruning
            // hidden directories as well prevents walking large .git and app
            // support trees only to discard their files later.
            .filter_entry(|entry| {
                entry.depth() == 0 || !entry.file_name().to_string_lossy().starts_with('.')
            });

        for entry in walker.filter_map(Result::ok) {
            let path = entry.path();

            // `path.is_file()` performs another metadata lookup. WalkDir has
            // already read the entry type, so use it before requesting the
            // metadata needed for size and date.
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let size_bytes = metadata.len();

                    if size_bytes >= size_threshold {
                        let name = entry.file_name().to_string_lossy().to_string();

                        // Skip hidden files
                        if name.starts_with('.') {
                            continue;
                        }

                        let extension = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_string();

                        let last_modified = metadata
                            .modified()
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

    // Sort by size descending for better UX
    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    items
}
