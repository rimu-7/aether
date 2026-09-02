use crate::models::application::Application;
use crate::models::artifact::Artifact;
use crate::services::{scanner, artifacts};
use crate::platform::utils::{is_safe_to_delete, is_protected_path, safe_delete_paths};
use std::path::Path;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::path::PathBuf;

#[tauri::command]
pub async fn scan_applications() -> Result<Vec<Application>, String> {
    Ok(scanner::scan_applications())
}

#[tauri::command]
pub async fn get_application_artifacts(bundle_id: Option<String>, app_name: String, bundle_path: String) -> Result<Vec<Artifact>, String> {
    Ok(artifacts::scan_application_artifacts(bundle_id.as_deref(), &app_name, &bundle_path))
}

#[tauri::command]
pub async fn get_app_icon(bundle_path: String, icon_path: Option<String>) -> Result<String, String> {
    let icon_path = icon_path.as_deref();
    #[cfg(target_os = "macos")]
    {
        let _ = icon_path;
        get_app_icon_macos(&bundle_path)
    }
    #[cfg(target_os = "linux")]
    {
        get_app_icon_linux(&bundle_path, icon_path.as_deref())
    }
    #[cfg(target_os = "windows")]
    {
        get_app_icon_windows(&bundle_path, icon_path.as_deref())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err("Unsupported platform".to_string())
    }
}

#[cfg(target_os = "macos")]
fn get_app_icon_macos(bundle_path: &str) -> Result<String, String> {
    let path = Path::new(bundle_path);
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
        .unwrap_or("AppIcon");

    let icon_name = if icon_file.ends_with(".icns") {
        icon_file.to_string()
    } else {
        format!("{}.icns", icon_file)
    };

    let icon_path = path.join("Contents/Resources").join(&icon_name);

    if !icon_path.exists() {
        return Err("Icon file not found".to_string());
    }

    let file = std::fs::File::open(&icon_path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    let icon_family = icns::IconFamily::read(reader).map_err(|e| e.to_string())?;

    let icon = icon_family.get_icon_with_type(icns::IconType::RGBA32_256x256)
        .or_else(|_| icon_family.get_icon_with_type(icns::IconType::RGBA32_128x128))
        .or_else(|_| icon_family.get_icon_with_type(icns::IconType::RGB24_128x128))
        .map_err(|e| e.to_string())?;

    let mut png_data = Vec::new();
    let img = image::RgbaImage::from_raw(icon.width(), icon.height(), icon.data().to_vec())
        .ok_or("Failed to create image from raw icon data")?;

    let mut cursor = std::io::Cursor::new(&mut png_data);
    img.write_to(&mut cursor, image::ImageFormat::Png).map_err(|e| e.to_string())?;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_data);
    Ok(format!("data:image/png;base64,{}", b64))
}

#[cfg(target_os = "linux")]
fn get_app_icon_linux(bundle_path: &str, icon_path: Option<&str>) -> Result<String, String> {
    // The bundle_path is the .desktop file path on Linux
    let desktop_path = Path::new(bundle_path);

    // First, try using icon_path if provided (resolved icon file path)
    if let Some(icon) = icon_path {
        let icon_file = Path::new(icon);
        if icon_file.exists() {
            let ext = icon_file.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "png" || ext == "xpm" || ext == "ico" || ext == "svg" {
                return read_image_as_png(icon_file);
            }
        }
    }

    // Parse the .desktop file for Icon= field
    if desktop_path.exists() && desktop_path.extension().and_then(|e| e.to_str()) == Some("desktop") {
        let content = std::fs::read_to_string(desktop_path).map_err(|e| e.to_string())?;

        let mut icon_name: Option<String> = None;
        for line in content.lines() {
            if line.starts_with("Icon=") && icon_name.is_none() {
                icon_name = Some(line.trim_start_matches("Icon=").trim().to_string());
                break;
            }
        }

        if let Some(icon_name) = icon_name {
            let resolved = resolve_linux_icon(&icon_name);
            if let Some(path) = resolved {
                return read_image_as_png(&path);
            }
        }
    }

    Err("Icon not found".to_string())
}

/// Search standard Freedesktop icon theme locations for an icon by name.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn resolve_linux_icon(icon_name: &str) -> Option<PathBuf> {
    // If it's an absolute path, use it directly
    let icon_path = PathBuf::from(icon_name);
    if icon_path.is_absolute() && icon_path.exists() {
        return Some(icon_path);
    }

    let home = dirs::home_dir()?;

    // Search paths
    let search_roots = vec![
        home.join(".local/share/icons"),
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
        PathBuf::from("/usr/share/icons/hicolor"),
        home.join(".icons"),
    ];

    // Common icon sizes to check
    let sizes = ["scalable", "256x256", "128x128", "64x64", "48x48", "32x32", "24x24", "16x16"];
    let extensions = ["png", "svg", "xpm"];

    // Try hicolor theme structure first: /usr/share/icons/hicolor/{size}/apps/{name}.{ext}
    for root in &search_roots {
        for size in &sizes {
            let apps_dir = root.join(size).join("apps");
            for ext in &extensions {
                let candidate = apps_dir.join(format!("{}.{}", icon_name, ext));
                if candidate.exists() {
                    // For SVG, we can't convert with the image crate, return the path
                    // The frontend can load SVG directly
                    if ext == &"svg" {
                        return Some(candidate);
                    }
                    return Some(candidate);
                }
            }
        }

        // Try root-level (e.g., /usr/share/pixmaps/{name}.{ext})
        for ext in &extensions {
            let candidate = root.join(format!("{}.{}", icon_name, ext));
            if candidate.exists() {
                if ext == &"svg" {
                    return Some(candidate);
                }
                return Some(candidate);
            }
        }
    }

    None
}

/// Read an image file (PNG/ICO/BMP/XPM/SVG) and return as a base64-encoded data URL.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn read_image_as_png(path: &Path) -> Result<String, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if ext == "svg" {
        // SVG is text-based; embed as a data URL (no rasterization needed, webview renders SVG natively)
        let svg_content = std::fs::read(path).map_err(|e| e.to_string())?;
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&svg_content);
        return Ok(format!("data:image/svg+xml;base64,{}", b64));
    }

    let img = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;

    let mut png_data = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_data);
    img.write_to(&mut cursor, image::ImageFormat::Png).map_err(|e| e.to_string())?;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_data);
    Ok(format!("data:image/png;base64,{}", b64))
}

#[cfg(target_os = "windows")]
fn get_app_icon_windows(bundle_path: &str, icon_path: Option<&str>) -> Result<String, String> {
    // bundle_path is typically the install location (directory)
    // icon_path may contain the DisplayIcon registry value

    let bundle = Path::new(bundle_path);

    // First, try using icon_path (DisplayIcon value from registry)
    if let Some(icon) = icon_path {
        // DisplayIcon format: "C:\path\to\app.exe,0" or "C:\path\to\icon.ico,0"
        let icon_str = icon.split(',').next().unwrap_or(icon).trim();
        let icon_file = Path::new(icon_str);

        if icon_file.exists() {
            let ext = icon_file.extension().and_then(|e| e.to_str()).unwrap_or("");

            if ext == "ico" || ext == "png" || ext == "bmp" {
                return read_image_as_png(icon_file);
            }

            if ext == "exe" || ext == "dll" {
                // Extract icon using PowerShell
                return extract_icon_from_exe_powershell(icon_file);
            }
        }
    }

    // Try to find .ico or .png files in the install directory
    if bundle.exists() && bundle.is_dir() {
        if let Ok(entries) = std::fs::read_dir(bundle) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "ico" || ext == "png" {
                        if let Ok(result) = read_image_as_png(&path) {
                            return Ok(result);
                        }
                    }
                }
            }
        }
    }

    // Try to find an .exe in the install directory and extract its icon
    if bundle.exists() && bundle.is_dir() {
        if let Ok(entries) = std::fs::read_dir(bundle) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "exe" {
                        if let Ok(result) = extract_icon_from_exe_powershell(&path) {
                            return Ok(result);
                        }
                    }
                }
            }
        }
    }

    Err("Icon not found".to_string())
}

/// Extract an icon from an executable using PowerShell and System.Drawing.
#[cfg(target_os = "windows")]
fn extract_icon_from_exe_powershell(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_string_lossy().to_string();

    // PowerShell script to extract the associated icon from an exe.
    // Uses single-quoted strings in PowerShell; single quotes in the path
    // are escaped as '' (PowerShell's escape sequence).
    let ps_script = format!(
        r#"
try {{
    Add-Type -AssemblyName System.Drawing -ErrorAction SilentlyContinue
    $icon = [System.Drawing.Icon]::ExtractAssociatedIcon('{0}')
    if ($icon -ne $null) {{
        $bitmap = $icon.ToBitmap()
        $ms = New-Object System.IO.MemoryStream
        $bitmap.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        [Convert]::ToBase64String($ms.ToArray())
    }}
}} catch {{ }}
"#,
        exe_str.replace('\'', "''")
    );

    let output = std::process::Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let b64 = stdout.trim();

    if b64.is_empty() {
        return Err("Failed to extract icon from executable".to_string());
    }

    Ok(format!("data:image/png;base64,{}", b64))
}

#[tauri::command]
pub async fn delete_artifacts(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut deleted = Vec::new();

    let allowed_prefixes = safe_delete_paths();

    for path_str in paths {
        let p = Path::new(&path_str);

        if is_protected_path(&path_str) {
            println!("Safety engine blocked deletion of protected root: {}", path_str);
            continue;
        }

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
