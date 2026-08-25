use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_type: String,
    pub os_version: String,
    pub architecture: String,
    pub hostname: String,
    pub total_memory: u64,
    pub free_memory: u64,
    pub total_swap: u64,
    pub free_swap: u64,
    pub uptime: u64,
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub disk_total: u64,
    pub disk_free: u64,
    pub network_tx: u64,
    pub network_rx: u64,
    pub is_laptop: bool,
    pub battery_percentage: f32,
    pub process_count: usize,
    pub kernel_version: String,
    pub os_build: String,
    pub cpu_frequency: u64,
    pub load_average_1m: f64,
    pub load_average_5m: f64,
    pub load_average_15m: f64,
}
