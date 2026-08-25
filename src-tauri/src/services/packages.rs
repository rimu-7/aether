use std::process::Command;
use serde_json::Value;
use crate::models::package::Package;

pub fn get_installed_packages() -> Result<Vec<Package>, String> {
    let mut packages = Vec::new();

    if cfg!(target_os = "macos") {
        // macOS: Use explicit paths for brew since GUI apps might not have it in PATH
        let brew_path = if std::path::Path::new("/opt/homebrew/bin/brew").exists() {
            "/opt/homebrew/bin/brew"
        } else if std::path::Path::new("/usr/local/bin/brew").exists() {
            "/usr/local/bin/brew"
        } else {
            "brew"
        };
        
        if let Ok(output) = Command::new(brew_path).args(&["info", "--json=v2", "--installed"]).output() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                if let Some(formulae) = parsed.get("formulae").and_then(|f| f.as_array()) {
                    for f in formulae {
                        let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                        let desc = f.get("desc").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let version = f.get("installed").and_then(|i| i.as_array()).and_then(|arr| arr.first()).and_then(|inst| inst.get("version")).and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                        packages.push(Package { id: name.clone(), name, description: desc, version, is_cask: false });
                    }
                }
                if let Some(casks) = parsed.get("casks").and_then(|c| c.as_array()) {
                    for c in casks {
                        let token = c.get("token").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                        let name = c.get("name").and_then(|v| v.as_array()).and_then(|arr| arr.first()).and_then(|v| v.as_str()).unwrap_or(&token).to_string();
                        let desc = c.get("desc").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let version = c.get("version").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                        packages.push(Package { id: token, name, description: desc, version, is_cask: true });
                    }
                }
            }
        }
    } else if cfg!(target_os = "linux") {
        // Try dnf first (Fedora)
        if let Ok(output) = Command::new("rpm").args(&["-qa", "--qf", "%{NAME}|%{VERSION}|%{SUMMARY}\n"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.splitn(3, '|').collect();
                    if parts.len() == 3 {
                        packages.push(Package {
                            id: parts[0].to_string(),
                            name: parts[0].to_string(),
                            version: parts[1].to_string(),
                            description: Some(parts[2].to_string()),
                            is_cask: false,
                        });
                    }
                }
                return Ok(packages); // Return if rpm succeeds
            }
        }
        
        // Try dpkg (Debian/Ubuntu)
        if let Ok(output) = Command::new("dpkg-query").args(&["-W", "-f=${Package}|${Version}|${Description}\n"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.splitn(3, '|').collect();
                    if parts.len() >= 2 {
                        let name = parts[0].to_string();
                        let version = parts[1].to_string();
                        let desc = parts.get(2).map(|s| s.replace('\n', " ").trim().to_string());
                        packages.push(Package { id: name.clone(), name, version, description: desc, is_cask: false });
                    }
                }
            }
        }
    } else if cfg!(target_os = "windows") {
        // Use PowerShell to read registry for installed programs
        if let Ok(output) = Command::new("powershell")
            .args(&["-NoProfile", "-Command", r#"
                Get-ItemProperty HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*, HKLM:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall\* -ErrorAction SilentlyContinue | 
                Where-Object { $_.DisplayName -ne $null } | 
                Select-Object DisplayName, DisplayVersion, Publisher | 
                ConvertTo-Json -Compress
            "#])
            .output() {
            
            if let Ok(json_str) = String::from_utf8(output.stdout) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                    if let Some(arr) = parsed.as_array() {
                        for item in arr {
                            if let Some(name) = item.get("DisplayName").and_then(|v| v.as_str()) {
                                let version = item.get("DisplayVersion").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                                let desc = item.get("Publisher").and_then(|v| v.as_str()).map(|s| s.to_string());
                                packages.push(Package {
                                    id: name.to_string(),
                                    name: name.to_string(),
                                    version,
                                    description: desc,
                                    is_cask: false,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(packages)
}

pub fn uninstall_package(id: &str, is_cask: bool) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        let brew_path = if std::path::Path::new("/opt/homebrew/bin/brew").exists() {
            "/opt/homebrew/bin/brew"
        } else if std::path::Path::new("/usr/local/bin/brew").exists() {
            "/usr/local/bin/brew"
        } else {
            "brew"
        };
        let mut cmd = Command::new(brew_path);
        cmd.arg("uninstall");
        if is_cask { cmd.arg("--cask"); }
        cmd.arg(id);
        
        let output = cmd.output().map_err(|e| format!("Failed to execute brew uninstall: {}", e))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
    } else if cfg!(target_os = "linux") {
        // Try pkexec for admin privileges to uninstall
        let status = Command::new("pkexec").args(&["dnf", "remove", "-y", id]).status();
        if status.is_err() || !status.unwrap().success() {
            let apt_status = Command::new("pkexec").args(&["apt-get", "remove", "-y", id]).status();
            if apt_status.is_err() || !apt_status.unwrap().success() {
                return Err("Failed to uninstall package via dnf or apt.".to_string());
            }
        }
    } else if cfg!(target_os = "windows") {
        let output = Command::new("powershell")
            .args(&["-NoProfile", "-Command", &format!("Start-Process -Wait -FilePath wmic.exe -ArgumentList \"product where name='{}' call uninstall /nointeractive\" -Verb RunAs", id)])
            .output()
            .map_err(|e| e.to_string())?;
            
        if !output.status.success() {
            return Err("Failed to uninstall package on Windows.".to_string());
        }
    }
    
    Ok(())
}
