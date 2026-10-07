use crate::models::cleaner::CleanableItem;
use crate::models::deletion::DeletionResult;
use crate::platform::utils::{
    cleanable_targets, is_protected_path, is_safe_to_delete, reveal_in_file_manager,
};
use crate::services::cleaner;
use std::path::Path;

#[tauri::command]
pub async fn scan_cleanable_items() -> Result<Vec<CleanableItem>, String> {
    tauri::async_runtime::spawn_blocking(cleaner::scan_cleanable_items)
        .await
        .map_err(|error| format!("Cleaner scan did not complete: {error}"))
}

#[tauri::command]
pub async fn delete_cleanable_items(paths: Vec<String>) -> Result<DeletionResult, String> {
    let mut result = DeletionResult::default();

    let allowed_prefixes: Vec<std::path::PathBuf> = cleanable_targets()
        .into_iter()
        .map(|(path, _)| path)
        .collect();

    for path_str in paths {
        let p = Path::new(&path_str);

        // Safety Block: Never allow deleting protected system paths
        if is_protected_path(&path_str) {
            println!(
                "Safety engine blocked deletion of protected root: {}",
                path_str
            );
            result.failed(path_str, "This is a protected system or profile root.");
            continue;
        }

        // Ensure the path is within an allowed cleanable directory
        if !is_safe_to_delete(p, &allowed_prefixes) {
            println!(
                "Safety engine blocked deletion of unauthorized path: {}",
                path_str
            );
            result.failed(
                path_str,
                "This location is not a current cache or temporary-file target.",
            );
            continue;
        }

        if p.exists() {
            match trash::delete(p) {
                Ok(_) => {
                    println!("Successfully moved to trash: {}", path_str);
                    result.deleted.push(path_str);
                }
                Err(error) => {
                    println!("Failed to move {} to trash: {}", path_str, error);
                    result.failed(path_str, format!("Could not move to Trash: {error}"));
                }
            }
        } else {
            println!("Path does not exist, cannot delete: {}", path_str);
            result.failed(path_str, "The item no longer exists.");
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn reveal_in_finder(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    reveal_in_file_manager(p)
}
