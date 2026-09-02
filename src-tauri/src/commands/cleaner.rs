use crate::models::cleaner::CleanableItem;
use crate::services::cleaner;
use crate::platform::utils::{is_safe_to_delete, is_protected_path, reveal_in_file_manager, cleanable_targets};
use std::path::Path;

#[tauri::command]
pub async fn scan_cleanable_items() -> Result<Vec<CleanableItem>, String> {
    Ok(cleaner::scan_cleanable_items())
}

#[tauri::command]
pub async fn delete_cleanable_items(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut deleted = Vec::new();

    let allowed_prefixes: Vec<std::path::PathBuf> = cleanable_targets()
        .into_iter()
        .map(|(path, _)| path)
        .collect();

    for path_str in paths {
        let p = Path::new(&path_str);

        // Safety Block: Never allow deleting protected system paths
        if is_protected_path(&path_str) {
            println!("Safety engine blocked deletion of protected root: {}", path_str);
            continue;
        }

        // Ensure the path is within an allowed cleanable directory
        if !is_safe_to_delete(p, &allowed_prefixes) {
            println!("Safety engine blocked deletion of unauthorized path: {}", path_str);
            continue;
        }

        if p.exists() {
            match trash::delete(p) {
                Ok(_) => {
                    println!("Successfully moved to trash: {}", path_str);
                    deleted.push(path_str);
                }
                Err(e) => println!("Failed to move {} to trash: {}", path_str, e),
            }
        } else {
            println!("Path does not exist, cannot delete: {}", path_str);
        }
    }

    Ok(deleted)
}

#[tauri::command]
pub async fn reveal_in_finder(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    reveal_in_file_manager(p)
}
