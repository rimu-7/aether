use crate::models::file_item::FileItem;
use crate::services::files;
use crate::platform::utils::{is_safe_to_delete, is_protected_path, user_scannable_dirs};
use std::path::Path;

#[tauri::command]
pub async fn scan_large_files() -> Result<Vec<FileItem>, String> {
    Ok(files::scan_large_files())
}

#[tauri::command]
pub async fn delete_files(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut deleted = Vec::new();

    let allowed_prefixes: Vec<std::path::PathBuf> = user_scannable_dirs()
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

        // Ensure the path is within an allowed user directory (Downloads, Documents, etc.)
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
        }
    }

    Ok(deleted)
}
