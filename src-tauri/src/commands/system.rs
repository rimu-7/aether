use crate::models::system::SystemInfo;
use sysinfo::{System, Disks, Networks};

#[tauri::command]
pub fn get_system_info() -> Result<SystemInfo, String> {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let os_type = System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let architecture = System::cpu_arch();
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
    
    let cpu_brand = sys.cpus().first().map(|cpu| cpu.brand().to_string()).unwrap_or_else(|| "Unknown".to_string());
    let cpu_cores = sys.cpus().len();
    
    // Disks
    let disks = Disks::new_with_refreshed_list();
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
    let networks = Networks::new_with_refreshed_list();
    let mut network_tx = 0;
    let mut network_rx = 0;
    for (_, network) in networks.iter() {
        network_tx += network.total_transmitted();
        network_rx += network.total_received();
    }
    
    // Battery
    let mut is_laptop = false;
    let mut battery_percentage = 0.0;
    
    // starship/mac battery lookup via IOkit is tricky in rust without specific crates,
    // but sysinfo actually doesn't have battery in System anymore, or it does?
    // Wait, sysinfo has `sys.batteries()` but we need to check if it's available.
    // Actually, `sysinfo` no longer provides batteries natively in the newer versions. It was moved or removed.
    // Let's use macOS specific command `pmset -g batt` if needed, or just default to false for now,
    // but wait! `battery` crate exists, but without it we can just call `pmset -g batt` via Command.
    use std::process::Command;
    let batt_output = Command::new("pmset")
        .arg("-g")
        .arg("batt")
        .output();
        
    if let Ok(output) = batt_output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Battery") || stdout.contains("InternalBattery") {
            is_laptop = true;
            // Parse percentage: "100%;"
            if let Some(pct_str) = stdout.split('%').next() {
                if let Some(val_str) = pct_str.split_whitespace().last() {
                    if let Ok(val) = val_str.parse::<f32>() {
                        battery_percentage = val;
                    }
                }
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
