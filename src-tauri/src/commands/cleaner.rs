use crate::models::cleaner::CleanableItem;
use crate::services::cleaner;
use std::process::Command;

#[tauri::command]
pub async fn scan_cleanable_items() -> Result<Vec<CleanableItem>, String> {
    // This can be slow, but async tauri commands run on a threadpool
    Ok(cleaner::scan_cleanable_items())
}

#[tauri::command]
pub async fn delete_cleanable_items(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut deleted = Vec::new();
    
    // Minimal safety check
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
        
    for path_str in paths {
        let p = std::path::Path::new(&path_str);
        
        // Safety Block: Never allow deleting root or major system folders
        if path_str == "/" || path_str == "/Applications" || path_str == home || 
           path_str == format!("{}/Library", home) || path_str == "/System" || 
           path_str == "/System/Applications" {
            println!("Safety engine blocked deletion of protected root: {}", path_str);
            continue;
        }
        
        // Ensure we're only deleting from Caches or Logs
        let allowed_cache = format!("{}/Library/Caches", home);
        let allowed_logs = format!("{}/Library/Logs", home);
        
        if !(path_str.starts_with(&allowed_cache) || path_str.starts_with(&allowed_logs)) {
            println!("Safety engine blocked deletion of unauthorized path: {}", path_str);
            continue;
        }
        
        // Don't delete the Caches or Logs directory itself!
        if path_str == allowed_cache || path_str == allowed_logs {
            continue;
        }
        
        if p.exists() {
            match trash::delete(p) {
                Ok(_) => {
                    println!("Successfully moved to trash: {}", path_str);
                    deleted.push(path_str);
                },
                Err(e) => println!("Failed to move {} to trash: {}", path_str, e),
            }
        }
    }
    
    Ok(deleted)
}

#[tauri::command]
pub async fn reveal_in_finder(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err("Path does not exist".to_string());
    }
    
    Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn()
        .map_err(|e| format!("Failed to open Finder: {}", e))?;
        
    Ok(())
}
