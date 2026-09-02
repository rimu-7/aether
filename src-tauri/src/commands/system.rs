use crate::models::system::SystemInfo;
use sysinfo::System;
use std::sync::Arc;
use crate::AppState;

#[tauri::command]
pub fn get_system_info(state: tauri::State<'_, Arc<AppState>>) -> Result<SystemInfo, String> {
    let mut sys = state.sys.lock().map_err(|e| e.to_string())?;
    sys.refresh_all();
    
    let os_type = System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let architecture = System::cpu_arch();
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
    
    let cpu_brand = sys.cpus().first().map(|cpu| cpu.brand().to_string()).unwrap_or_else(|| "Unknown".to_string());
    let cpu_cores = sys.cpus().len();
    
    // Disks
    let mut disks = state.disks.lock().map_err(|e| e.to_string())?;
    disks.refresh(true);
    let mut disk_total = 0;
    let mut disk_free = 0;
    
    // Find the primary system drive to avoid double-counting synthesized APFS/BTRFS volumes
    let root_path = if cfg!(target_os = "windows") {
        std::path::Path::new("C:\\")
    } else {
        std::path::Path::new("/")
    };

    if let Some(main_disk) = disks.list().iter().find(|d| d.mount_point() == root_path) {
        disk_total = main_disk.total_space();
        disk_free = main_disk.available_space();
    } else if let Some(first_disk) = disks.list().first() {
        disk_total = first_disk.total_space();
        disk_free = first_disk.available_space();
    }
    
    // Networks
    let networks = state.networks.lock().map_err(|e| e.to_string())?;
    let mut network_tx = 0;
    let mut network_rx = 0;
    for (_, network) in networks.iter() {
        network_tx += network.total_transmitted();
        network_rx += network.total_received();
    }
    
    // Battery
    let mut is_laptop = false;
    let mut battery_percentage = 0.0;
    
    if cfg!(target_os = "macos") {
        use std::process::Command;
        if let Ok(output) = Command::new("pmset").arg("-g").arg("batt").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("Battery") || stdout.contains("InternalBattery") {
                is_laptop = true;
                if let Some(pct_str) = stdout.split('%').next() {
                    if let Some(val_str) = pct_str.split_whitespace().last() {
                        if let Ok(val) = val_str.parse::<f32>() {
                            battery_percentage = val;
                        }
                    }
                }
            }
        }
    } else if cfg!(target_os = "linux") {
        // Check for any battery device in /sys/class/power_supply/
        if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("BAT") {
                    if let Ok(capacity) = std::fs::read_to_string(entry.path().join("capacity")) {
                        is_laptop = true;
                        if let Ok(val) = capacity.trim().parse::<f32>() {
                            battery_percentage = val;
                        }
                        break;
                    }
                }
            }
        }
    } else if cfg!(target_os = "windows") {
        use std::process::Command;
        // Use Get-CimInstance (modern replacement for Get-WmiObject)
        if let Ok(output) = Command::new("powershell")
            .args(&["-NoProfile", "-Command", r#"
                $batteries = Get-CimInstance -ClassName Win32_Battery -ErrorAction SilentlyContinue
                if ($batteries -ne $null) {
                    $b = $batteries[0]
                    Write-Output $b.EstimatedChargeRemaining
                }
            "#])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(val) = stdout.trim().parse::<f32>() {
                is_laptop = true;
                battery_percentage = val;
            }
        }
    }
    
    Ok(SystemInfo {
        os_type,
        os_version,
        architecture,
        hostname,
        total_memory: sys.total_memory(),
        free_memory: sys.total_memory().saturating_sub(sys.used_memory()),
        total_swap: sys.total_swap(),
        free_swap: sys.free_swap(),
        uptime: System::uptime(),
        cpu_brand,
        cpu_cores,
        disk_total,
        disk_free,
        network_tx,
        network_rx,
        is_laptop,
        battery_percentage,
        process_count: sys.processes().len(),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
        os_build: System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
        cpu_frequency: sys.cpus().first().map(|cpu| cpu.frequency()).unwrap_or(0),
        load_average_1m: System::load_average().one,
        load_average_5m: System::load_average().five,
        load_average_15m: System::load_average().fifteen,
    })
}
