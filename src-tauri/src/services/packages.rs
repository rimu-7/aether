use std::process::Command;
use std::path::Path;
use serde_json::Value;
use crate::models::package::Package;

/// Detect which package manager is available on this Linux system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageManager {
    Dnf,      // Fedora, RHEL, CentOS
    Apt,      // Debian, Ubuntu
    Pacman,   // Arch, Manjaro
    Zypper,   // openSUSE
    Emerge,   // Gentoo
    Xbps,     // Void Linux
}

impl PackageManager {
    fn detect() -> Option<Self> {
        for (pm, cmd) in [
            (PackageManager::Dnf, "dnf"),
            (PackageManager::Apt, "dpkg-query"),
            (PackageManager::Pacman, "pacman"),
            (PackageManager::Zypper, "zypper"),
            (PackageManager::Emerge, "eix"),
            (PackageManager::Xbps, "xbps-query"),
        ] {
            if Path::new("/usr/bin").join(cmd).exists()
                || Path::new("/usr/local/bin").join(cmd).exists()
                || Command::new("sh").arg("-c").arg(format!("command -v {}", cmd)).output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            {
                return Some(pm);
            }
        }
        None
    }
}

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
        match PackageManager::detect() {
            Some(PackageManager::Dnf) => {
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
                    }
                }
            }
            Some(PackageManager::Apt) => {
                if let Ok(output) = Command::new("dpkg-query").args(&["-W", "-f=${Package}|${Version}|${binary:Summary}\n"]).output() {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        for line in stdout.lines() {
                            let parts: Vec<&str> = line.splitn(3, '|').collect();
                            if parts.len() >= 2 {
                                let name = parts[0].to_string();
                                let version = parts[1].to_string();
                                let desc = parts.get(2).map(|s| s.replace('\n', " ").trim().to_string());
                                packages.push(Package {
                                    id: name.clone(),
                                    name,
                                    version,
                                    description: desc,
                                    is_cask: false,
                                });
                            }
                        }
                    }
                }
            }
            Some(PackageManager::Pacman) => {
                if let Ok(output) = Command::new("pacman").args(&["-Q"]).output() {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        for line in stdout.lines() {
                            let parts: Vec<&str> = line.splitn(2, ' ').collect();
                            if parts.len() == 2 {
                                packages.push(Package {
                                    id: parts[0].to_string(),
                                    name: parts[0].to_string(),
                                    version: parts[1].to_string(),
                                    description: None,
                                    is_cask: false,
                                });
                            }
                        }
                    }
                }
            }
            Some(PackageManager::Zypper) => {
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
                    }
                }
            }
            Some(PackageManager::Xbps) => {
                if let Ok(output) = Command::new("xbps-query").args(&["-s", ""]).output() {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        for line in stdout.lines() {
                            let trimmed = line.trim_start();
                            let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                            if parts.len() == 2 {
                                packages.push(Package {
                                    id: parts[0].to_string(),
                                    name: parts[0].to_string(),
                                    version: parts[1].to_string(),
                                    description: None,
                                    is_cask: false,
                                });
                            }
                        }
                    }
                }
            }
            Some(PackageManager::Emerge) => {
                if let Ok(output) = Command::new("eix").args(&["-c", "--no-color"]).output() {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        for line in stdout.lines() {
                            let line = line.trim();
                            if line.starts_with('[') {
                                // Parse eix output: [1] app-category/name ...
                                let after_bracket = line.split("] ").nth(1).unwrap_or("");
                                let parts: Vec<&str> = after_bracket.splitn(3, ' ').collect();
                                if parts.len() >= 3 {
                                    packages.push(Package {
                                        id: parts[1].to_string(),
                                        name: parts[1].to_string(),
                                        version: parts[2].to_string(),
                                        description: None,
                                        is_cask: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            None => {
                // No known package manager detected
            }
        }
    } else if cfg!(target_os = "windows") {
        // Use PowerShell to read registry for installed programs
        if let Ok(output) = Command::new("powershell")
            .args(&["-NoProfile", "-Command", r#"
                Get-ItemProperty HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*, HKLM:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*, HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\* -ErrorAction SilentlyContinue |
                Where-Object { $_.DisplayName -ne $null } |
                Select-Object DisplayName, DisplayVersion, Publisher, DisplayIcon |
                ConvertTo-Json -Compress
            "#])
            .output()
        {
            if let Ok(json_str) = String::from_utf8(output.stdout) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                    if let Some(arr) = parsed.as_array() {
                        for item in arr {
                            if let Some(name) = item.get("DisplayName").and_then(|v| v.as_str()) {
                                let version = item.get("DisplayVersion").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                                let desc = item.get("Publisher").and_then(|v| v.as_str()).map(|s| s.to_string());
                                // is_cask = true if the app has a DisplayIcon (GUI application)
                                let is_cask = item.get("DisplayIcon").is_some();
                                packages.push(Package {
                                    id: name.to_string(),
                                    name: name.to_string(),
                                    version,
                                    description: desc,
                                    is_cask,
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
        // Detect package manager and uninstall accordingly
        match PackageManager::detect() {
            Some(pm) => {
                let result = match pm {
                    PackageManager::Dnf => {
                        Command::new("pkexec").args(&["dnf", "remove", "-y", id]).status()
                    }
                    PackageManager::Apt => {
                        Command::new("pkexec").args(&["apt-get", "remove", "-y", id]).status()
                    }
                    PackageManager::Pacman => {
                        Command::new("pkexec").args(&["pacman", "-R", "--noconfirm", id]).status()
                    }
                    PackageManager::Zypper => {
                        Command::new("pkexec").args(&["zypper", "-n", "remove", id]).status()
                    }
                    PackageManager::Emerge => {
                        Command::new("pkexec").args(&["emerge", "-C", id]).status()
                    }
                    PackageManager::Xbps => {
                        Command::new("pkexec").args(&["xbps-remove", "-y", id]).status()
                    }
                };

                match result {
                    Ok(status) if status.success() => {}
                    Ok(status) => {
                        return Err(format!("Failed to uninstall package: exit code {:?}", status.code()));
                    }
                    Err(e) => {
                        return Err(format!("Failed to execute uninstall: {}", e));
                    }
                }
            }
            None => {
                return Err("No supported package manager found on this system.".to_string());
            }
        }
    } else if cfg!(target_os = "windows") {
        // Use the registry UninstallString for reliable, fast uninstallation.
        // Win32_Product is avoided because it triggers a consistency check
        // on all installed packages and is very slow.
        let ps_script = format!(
            r#"
$regPaths = @(
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*"
)
$found = $false
foreach ($regPath in $regPaths) {{
    $item = Get-ItemProperty $regPath -ErrorAction SilentlyContinue | Where-Object {{ $_.DisplayName -eq '{0}' }}
    if ($item -ne $null -and $item.UninstallString -ne $null) {{
        $found = $true
        $uninstallCmd = $item.UninstallString
        try {{
            # Run silently
            Invoke-Expression "$uninstallCmd /S" 2>$null
            if ($LASTEXITCODE -ne 0) {{
                # Try without /S flag
                Invoke-Expression $uninstallCmd 2>$null
            }}
        }} catch {{
            # Fall back to msiexec
            $guid = $item.PSChildName
            if ($guid -match '{{.}}') {{
                Start-Process -Wait -FilePath msiexec.exe -ArgumentList "/x $($guid) /quiet" -Verb RunAs
            }} else {{
                Start-Process -Wait -FilePath $uninstallCmd -Verb RunAs
            }}
        }}
    }}
}}
if (-not $found) {{
    Write-Error "Package '{0}' not found in registry"
    exit 1
}}
"#,
            id.replace('\'', "''")
        );

        let output = Command::new("powershell")
            .args(&["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to uninstall package: {}", stderr.trim()));
        }
    }

    Ok(())
}
