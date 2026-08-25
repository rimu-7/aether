export interface SystemInfo {
  os_type: string;
  os_version: string;
  architecture: string;
  hostname: string;
  total_memory: number;
  free_memory: number;
  total_swap: number;
  free_swap: number;
  uptime: number;
  cpu_brand: string;
  cpu_cores: number;
  disk_total: number;
  disk_free: number;
  network_tx: number;
  network_rx: number;
  is_laptop: boolean;
  battery_percentage: number;
  process_count: number;
  kernel_version: string;
  os_build: string;
  cpu_frequency: number;
  load_average_1m: number;
  load_average_5m: number;
  load_average_15m: number;
}
