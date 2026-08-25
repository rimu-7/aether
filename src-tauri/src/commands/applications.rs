use crate::models::application::Application;
use crate::models::artifact::Artifact;
use crate::services::{scanner, artifacts};

#[tauri::command]
pub async fn scan_applications() -> Result<Vec<Application>, String> {
    // Doing it synchronously here for simplicity, but could be offloaded to a thread
    Ok(scanner::scan_applications())
}

#[tauri::command]
pub async fn get_application_artifacts(bundle_id: Option<String>, app_name: String) -> Result<Vec<Artifact>, String> {
    Ok(artifacts::scan_application_artifacts(bundle_id.as_deref(), &app_name))
}

#[tauri::command]
pub async fn get_app_icon(bundle_path: String) -> Result<String, String> {
    let path = std::path::Path::new(&bundle_path);
    let info_plist = path.join("Contents/Info.plist");
    
    if !info_plist.exists() {
        return Err("No Info.plist found".to_string());
    }
    
    let dict = plist::Value::from_file(&info_plist)
        .map_err(|e| e.to_string())?;
        
    let dict = dict.as_dictionary()
        .ok_or("Info.plist is not a dictionary")?;
        
    let icon_file = dict.get("CFBundleIconFile")
        .and_then(|v| v.as_string())
        .unwrap_or("AppIcon"); // Fallback
        
    // The icon file might or might not have the .icns extension
    let icon_name = if icon_file.ends_with(".icns") {
        icon_file.to_string()
    } else {
        format!("{}.icns", icon_file)
    };
    
    let icon_path = path.join("Contents/Resources").join(&icon_name);
    
    if !icon_path.exists() {
        return Err("Icon file not found".to_string());
    }
    
    // Read the ICNS file
    let file = std::fs::File::open(&icon_path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    let icon_family = icns::IconFamily::read(reader).map_err(|e| e.to_string())?;
    
    // Try to get a high-quality icon (e.g. 256x256 or 128x128)
    let icon = icon_family.get_icon_with_type(icns::IconType::RGBA32_256x256)
        .or_else(|_| icon_family.get_icon_with_type(icns::IconType::RGBA32_128x128))
        .or_else(|_| icon_family.get_icon_with_type(icns::IconType::RGB24_128x128))
        .map_err(|e| e.to_string())?;
        
    // Convert to PNG
    let mut png_data = Vec::new();
    let img = image::RgbaImage::from_raw(icon.width(), icon.height(), icon.data().to_vec())
        .ok_or("Failed to create image from raw icon data")?;
        
    let mut cursor = std::io::Cursor::new(&mut png_data);
    img.write_to(&mut cursor, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_data);
    Ok(format!("data:image/png;base64,{}", b64))
}

#[tauri::command]
pub async fn delete_artifacts(paths: Vec<String>) -> Result<Vec<String>, String> {
    // SAFETY ENGINE: Super minimal root check
    let mut deleted = Vec::new();
    let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    
    for path_str in paths {
        let p = std::path::Path::new(&path_str);
        
        // Safety Block: Never allow deleting root or major system folders
        if path_str == "/" || path_str == "/Applications" || path_str == home || 
           path_str == format!("{}/Library", home) || path_str == "/System" || 
           path_str == "/System/Applications" {
            println!("Safety engine blocked deletion of protected root: {}", path_str);
            continue;
        }
        
        // Allow deletion if it's in the user's home directory OR in /Applications
        let mut allowed = false;
        if path_str.starts_with(&format!("{}/", home)) { allowed = true; }
        if path_str.starts_with("/Applications/") { allowed = true; }
        
        if !allowed {
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
        } else {
            println!("Path does not exist, cannot delete: {}", path_str);
        }
    }
    
    Ok(deleted)
}
