use crate::models::deletion::DeletionResult;
use crate::models::file_item::FileItem;
use crate::platform::utils::{
    is_protected_path, is_safe_selected_scan_root, is_safe_to_delete, user_scannable_dirs,
};
use crate::services::files;
use crate::AppState;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

#[tauri::command]
pub async fn scan_large_files(
    additional_paths: Option<Vec<String>>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<FileItem>, String> {
    let selected_roots: Vec<PathBuf> = additional_paths
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| is_safe_selected_scan_root(path))
        .collect();

    {
        let mut registered_roots = state
            .selected_file_roots
            .lock()
            .map_err(|_| "Could not register selected folders for this scan.".to_string())?;
        *registered_roots = selected_roots.clone();
    }

    tauri::async_runtime::spawn_blocking(move || files::scan_large_files(&selected_roots))
        .await
        .map_err(|error| format!("File scan did not complete: {error}"))
}

#[tauri::command]
pub async fn delete_files(
    paths: Vec<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<DeletionResult, String> {
    let mut result = DeletionResult::default();

    let mut allowed_prefixes: Vec<PathBuf> = user_scannable_dirs()
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    allowed_prefixes.extend(
        state
            .selected_file_roots
            .lock()
            .map_err(|_| "Could not verify selected folders for deletion.".to_string())?
            .iter()
            .cloned(),
    );

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

        // Ensure the path is within an allowed user directory (Downloads, Documents, etc.)
        if !is_safe_to_delete(p, &allowed_prefixes) {
            println!(
                "Safety engine blocked deletion of unauthorized path: {}",
                path_str
            );
            result.failed(path_str, "This location was not part of the current scan.");
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
            result.failed(path_str, "The file no longer exists.");
        }
    }

    Ok(result)
}
