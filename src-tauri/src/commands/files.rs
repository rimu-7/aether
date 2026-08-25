use crate::models::file_item::FileItem;
use crate::services::files;

#[tauri::command]
pub async fn scan_large_files() -> Result<Vec<FileItem>, String> {
    // This can be slow, but async tauri commands run on a threadpool
    Ok(files::scan_large_files())
}

#[tauri::command]
pub async fn delete_files(paths: Vec<String>) -> Result<Vec<String>, String> {
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
        
        // Ensure we're only deleting from designated folders
        let allowed_dirs = vec![
            format!("{}/Downloads", home),
            format!("{}/Documents", home),
            format!("{}/Desktop", home),
            format!("{}/Movies", home),
        ];
        
        let mut is_allowed = false;
        for dir in allowed_dirs {
            if path_str.starts_with(&dir) && path_str != dir {
                is_allowed = true;
                break;
            }
        }
        
        if !is_allowed {
            println!("Safety engine blocked deletion of unauthorized path: {}", path_str);
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
