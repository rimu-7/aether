use std::process::Command;
use serde_json::Value;
use crate::models::package::Package;

pub fn get_installed_packages() -> Result<Vec<Package>, String> {
    let output = Command::new("brew")
        .arg("info")
        .arg("--json=v2")
        .arg("--installed")
        .output()
        .map_err(|e| format!("Failed to execute brew: {}", e))?;

    if !output.status.success() {
        return Err("Brew command failed".to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse brew json: {}", e))?;

    let mut packages = Vec::new();

    // Parse Formulae
    if let Some(formulae) = parsed.get("formulae").and_then(|f| f.as_array()) {
        for f in formulae {
            let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
            let desc = f.get("desc").and_then(|v| v.as_str()).map(|s| s.to_string());
            
            // Get the installed version
            let version = f.get("installed")
                .and_then(|i| i.as_array())
                .and_then(|arr| arr.first())
                .and_then(|inst| inst.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            packages.push(Package {
                id: name.clone(),
                name,
                description: desc,
                version,
                is_cask: false,
            });
        }
    }

    // Parse Casks
    if let Some(casks) = parsed.get("casks").and_then(|c| c.as_array()) {
        for c in casks {
            let token = c.get("token").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
            
            // Try to get a nicer name if available
            let name = c.get("name")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or(&token)
                .to_string();

            let desc = c.get("desc").and_then(|v| v.as_str()).map(|s| s.to_string());
            let version = c.get("version").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();

            packages.push(Package {
                id: token,
                name,
                description: desc,
                version,
                is_cask: true,
            });
        }
    }

    Ok(packages)
}

pub fn uninstall_package(id: &str, is_cask: bool) -> Result<(), String> {
    let mut cmd = Command::new("brew");
    cmd.arg("uninstall");
    
    if is_cask {
        cmd.arg("--cask");
    }
    
    cmd.arg(id);
    
    let output = cmd.output().map_err(|e| format!("Failed to execute brew uninstall: {}", e))?;
    
    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Uninstall failed: {}", err_str));
    }
    
    Ok(())
}
