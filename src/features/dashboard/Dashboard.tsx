import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SystemInfo } from "@/types/system";
import {
  Monitor,
  Cpu,
  MemoryStick,
  HardDrive,
  Activity,
  Wifi,
  Battery,
  BatteryCharging,
} from "lucide-react";

export function Dashboard() {
  const [sysInfo, setSysInfo] = useState<SystemInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  // For calculating network speed
  const [downloadSpeed, setDownloadSpeed] = useState(0);
  const [uploadSpeed, setUploadSpeed] = useState(0);
  const lastNetworkState = useRef({ rx: 0, tx: 0, time: 0 });

  useEffect(() => {
    let isMounted = true;

    const fetchInfo = async () => {
      try {
        const info = await invoke<SystemInfo>("get_system_info");
        if (!isMounted) return;

        setSysInfo(info);

        const now = Date.now();
        if (lastNetworkState.current.time > 0) {
          const timeDiff = (now - lastNetworkState.current.time) / 1000;
          if (timeDiff > 0) {
            const rxDiff = info.network_rx - lastNetworkState.current.rx;
            const txDiff = info.network_tx - lastNetworkState.current.tx;
            setDownloadSpeed(Math.max(0, rxDiff / timeDiff));
            setUploadSpeed(Math.max(0, txDiff / timeDiff));
          }
        }

        lastNetworkState.current = {
          rx: info.network_rx,
          tx: info.network_tx,
          time: now,
        };
      } catch (e) {
        if (isMounted) setError(String(e));
      }
    };

    fetchInfo();
    const interval = setInterval(fetchInfo, 1000);

    return () => {
      isMounted = false;
      clearInterval(interval);
    };
  }, []);

  const formatBytes = (bytes: number) => {
    return (bytes / 1024 / 1024 / 1024).toFixed(1) + " GB";
  };

  const formatSpeed = (bytesPerSec: number) => {
    if (bytesPerSec === 0) return "0 B/s";
    const k = 1024;
    const sizes = ["B/s", "KB/s", "MB/s", "GB/s"];
    const i = Math.floor(Math.log(bytesPerSec) / Math.log(k));
    return (
      parseFloat((bytesPerSec / Math.pow(k, i)).toFixed(1)) + " " + sizes[i]
    );
  };

  return (
    <div className="flex flex-col gap-8 h-full overflow-y-auto min-h-0 pb-12 pr-4">
      <div>
        <h1 className="text-4xl font-bold tracking-tight shrink-0 bg-gradient-to-r from-primary to-primary/50 bg-clip-text text-transparent w-fit">
          Dashboard
        </h1>
        <p className="text-muted-foreground mt-1">
          Real-time telemetry and system diagnostics
        </p>
      </div>

      {error && (
        <div className="p-4 rounded-2xl bg-destructive/10 text-destructive border border-destructive/20 shadow-sm">
          <p className="font-medium">Failed to load system info</p>
          <p className="text-sm opacity-80">{error}</p>
        </div>
      )}

      {!sysInfo && !error && (
        <div className="grid grid-cols-1 md:grid-cols-12 gap-4 animate-pulse opacity-60">
          <div className="md:col-span-8 h-40 bg-muted rounded-3xl"></div>
          <div className="md:col-span-4 h-40 bg-muted rounded-3xl"></div>
          <div className="md:col-span-4 h-48 bg-muted rounded-3xl"></div>
          <div className="md:col-span-4 h-48 bg-muted rounded-3xl"></div>
          <div className="md:col-span-4 h-48 bg-muted rounded-3xl"></div>
        </div>
      )}

      {sysInfo && (
        <div className="grid grid-cols-1 md:grid-cols-12 gap-4 pb-12 auto-rows-fr">
          {/* OS Info - Hero Card */}
          <div className="md:col-span-7 lg:col-span-8 p-4 rounded-3xl border bg-gradient-to-br from-card to-card/50 text-card-foreground shadow-sm hover:shadow-md  transition-all duration-300 flex flex-col relative overflow-hidden group">
            <div className="absolute -right-12 -top-12 opacity-[0.03] group-hover:opacity-[0.05] transition-opacity duration-500">
              <Monitor size={200} />
            </div>
            <div className="flex items-center gap-3 mb-6">
              <div className="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center text-primary">
                <Monitor size={20} />
              </div>
              <h3 className="tracking-tight text-sm font-semibold text-primary uppercase">
                Operating System
              </h3>
            </div>
            <div className="mt-auto">
              <p
                className="text-4xl lg:text-5xl font-black tracking-tighter truncate"
                title={`${sysInfo.os_type} ${sysInfo.os_version}`}
              >
                {sysInfo.os_type}{" "}
                <span className="text-muted-foreground font-medium">
                  {sysInfo.os_version}
                </span>
              </p>
              <div className="flex flex-col gap-1 mt-2">
                <p
                  className="text-sm text-muted-foreground truncate font-medium"
                  title={sysInfo.hostname}
                >
                  Host: {sysInfo.hostname}
                </p>
                <p className="text-xs text-muted-foreground opacity-80 truncate">
                  Build: {sysInfo.os_build} • Kernel: {sysInfo.kernel_version}
                </p>
              </div>
            </div>
          </div>

          {/* CPU Info */}
          <div className="md:col-span-5 lg:col-span-4 p-4 rounded-3xl border bg-card text-card-foreground shadow-sm hover:shadow-md  transition-all duration-300 flex flex-col relative overflow-hidden">
            <div className="flex items-center gap-3 mb-6">
              <div className="w-10 h-10 rounded-full bg-blue-500/10 flex items-center justify-center text-blue-500">
                <Cpu size={20} />
              </div>
              <h3 className="tracking-tight text-sm font-semibold text-blue-500 uppercase">
                Processor
              </h3>
            </div>
            <div className="mt-auto">
              <p
                className="text-2xl font-bold truncate leading-tight"
                title={sysInfo.cpu_brand}
              >
                {sysInfo.cpu_brand}
              </p>
              <div className="flex gap-2 mt-3 flex-wrap">
                <span className="px-3 py-1 bg-muted rounded-full text-xs font-semibold">
                  {sysInfo.cpu_cores} Cores
                </span>
                <span className="px-3 py-1 bg-muted rounded-full text-xs font-semibold">
                  {sysInfo.architecture}
                </span>
                {sysInfo.cpu_frequency > 0 && (
                  <span className="px-3 py-1 bg-muted rounded-full text-xs font-semibold">
                    {(sysInfo.cpu_frequency / 1000).toFixed(2)} GHz
                  </span>
                )}
              </div>
            </div>
          </div>

          {/* Memory Info */}
          <div className="md:col-span-6 lg:col-span-4 p-4 rounded-3xl border bg-card text-card-foreground shadow-sm hover:shadow-md  transition-all duration-300 flex flex-col">
            <div className="flex items-center gap-3 mb-6">
              <div className="w-10 h-10 rounded-full bg-purple-500/10 flex items-center justify-center text-purple-500">
                <MemoryStick size={20} />
              </div>
              <h3 className="tracking-tight text-sm font-semibold text-purple-500 uppercase">
                Memory
              </h3>
            </div>
            <div className="mt-auto">
              <div className="flex items-end justify-between mb-2">
                <p className="text-3xl font-bold">
                  {formatBytes(sysInfo.total_memory - sysInfo.free_memory)}
                </p>
                <p className="text-sm text-muted-foreground font-medium mb-1">
                  / {formatBytes(sysInfo.total_memory)}
                </p>
              </div>
              <div className="w-full bg-muted rounded-full h-2.5 overflow-hidden">
                <div
                  className="bg-gradient-to-r from-purple-500 to-indigo-500 h-full rounded-full"
                  style={{
                    width: `${((sysInfo.total_memory - sysInfo.free_memory) / sysInfo.total_memory) * 100}%`,
                  }}
                ></div>
              </div>
            </div>
          </div>

          {/* Disk Info */}
          <div className="md:col-span-6 lg:col-span-4 p-4 rounded-3xl border bg-card text-card-foreground shadow-sm hover:shadow-md  transition-all duration-300 flex flex-col">
            <div className="flex items-center gap-3 mb-6">
              <div className="w-10 h-10 rounded-full bg-orange-500/10 flex items-center justify-center text-orange-500">
                <HardDrive size={20} />
              </div>
              <h3 className="tracking-tight text-sm font-semibold text-orange-500 uppercase">
                Storage
              </h3>
            </div>
            <div className="mt-auto">
              <div className="flex items-end justify-between mb-2">
                <p className="text-3xl font-bold">
                  {formatBytes(sysInfo.disk_total - sysInfo.disk_free)}
                </p>
                <p className="text-sm text-muted-foreground font-medium mb-1">
                  / {formatBytes(sysInfo.disk_total)}
                </p>
              </div>
              <div className="w-full bg-muted rounded-full h-2.5 overflow-hidden">
                <div
                  className="bg-gradient-to-r from-orange-500 to-red-500 h-full rounded-full"
                  style={{
                    width: `${((sysInfo.disk_total - sysInfo.disk_free) / Math.max(sysInfo.disk_total, 1)) * 100}%`,
                  }}
                ></div>
              </div>
            </div>
          </div>

          {/* Network Info */}
          <div className="md:col-span-12 lg:col-span-4 p-2 rounded-3xl border bg-card text-card-foreground shadow-sm hover:shadow-md  transition-all duration-300 flex flex-col">
            <div className="flex items-center gap-3 mb-6">
              <div className="w-10 h-10 rounded-full bg-cyan-500/10 flex items-center justify-center text-cyan-500">
                <Wifi size={20} />
              </div>
              <h3 className="tracking-tight text-sm font-semibold text-cyan-500 uppercase">
                Network
              </h3>
            </div>
            <div className="grid grid-cols-2 gap-4 mt-auto">
              <div className="bg-muted/50 p-4 rounded-2xl relative overflow-hidden group">
                <div className="absolute inset-0 bg-gradient-to-br from-cyan-500/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
                <p className="text-xs text-muted-foreground uppercase tracking-wider font-semibold mb-1">
                  Download
                </p>
                <p className="text-xl font-bold text-cyan-600 dark:text-cyan-400">
                  {formatSpeed(downloadSpeed)}
                </p>
                <p className="text-[10px] text-muted-foreground mt-1 opacity-70">
                  Total: {formatBytes(sysInfo.network_rx)}
                </p>
              </div>
              <div className="bg-muted/50 p-4 rounded-2xl relative overflow-hidden group">
                <div className="absolute inset-0 bg-gradient-to-br from-cyan-500/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
                <p className="text-xs text-muted-foreground uppercase tracking-wider font-semibold mb-1">
                  Upload
                </p>
                <p className="text-xl font-bold text-cyan-600 dark:text-cyan-400">
                  {formatSpeed(uploadSpeed)}
                </p>
                <p className="text-[10px] text-muted-foreground mt-1 opacity-70">
                  Total: {formatBytes(sysInfo.network_tx)}
                </p>
              </div>
            </div>
          </div>

          {/* System Status Info */}
          <div className="md:col-span-6 lg:col-span-6 p-4 rounded-3xl border bg-card text-card-foreground shadow-sm hover:shadow-md  transition-all duration-300 flex flex-col">
            <div className="flex items-center gap-3 mb-6">
              <div className="w-10 h-10 rounded-full bg-emerald-500/10 flex items-center justify-center text-emerald-500">
                <Activity size={20} />
              </div>
              <h3 className="tracking-tight text-sm font-semibold text-emerald-500 uppercase">
                Activity & Load
              </h3>
            </div>
            <div className="mt-auto grid grid-cols-3 gap-4">
              <div>
                <p className="text-xs text-muted-foreground uppercase tracking-wider font-semibold mb-1">
                  Tasks
                </p>
                <p className="text-2xl font-bold">
                  {sysInfo.process_count.toLocaleString()}
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground uppercase tracking-wider font-semibold mb-1">
                  Load (1m/5m)
                </p>
                <p className="text-2xl font-bold">
                  {sysInfo.load_average_1m.toFixed(2)}{" "}
                  <span className="text-sm font-medium text-muted-foreground">
                    / {sysInfo.load_average_5m.toFixed(2)}
                  </span>
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground uppercase tracking-wider font-semibold mb-1">
                  Uptime
                </p>
                <p className="text-xl font-bold mt-1">
                  {Math.floor(sysInfo.uptime / 86400) > 0 &&
                    `${Math.floor(sysInfo.uptime / 86400)}d `}
                  {Math.floor((sysInfo.uptime % 86400) / 3600)}h{" "}
                  {Math.floor((sysInfo.uptime % 3600) / 60)}m
                </p>
              </div>
            </div>
          </div>

          {/* Battery Info (Only if laptop) */}
          {sysInfo.is_laptop && (
            <div className="md:col-span-6 lg:col-span-6 p-4 rounded-3xl border bg-card text-card-foreground shadow-sm hover:shadow-md  transition-all duration-300 flex flex-col">
              <div className="flex items-center gap-3 mb-6">
                <div
                  className={`w-10 h-10 rounded-full flex items-center justify-center ${sysInfo.battery_percentage < 20 ? "bg-destructive/10 text-destructive" : "bg-green-500/10 text-green-500"}`}
                >
                  {sysInfo.battery_percentage < 100 ? (
                    <BatteryCharging size={20} />
                  ) : (
                    <Battery size={20} />
                  )}
                </div>
                <h3
                  className={`tracking-tight text-sm font-semibold uppercase ${sysInfo.battery_percentage < 20 ? "text-destructive" : "text-green-500"}`}
                >
                  Battery
                </h3>
              </div>
              <div className="mt-auto">
                <div className="flex items-end justify-between mb-2">
                  <p className="text-3xl font-bold">
                    {sysInfo.battery_percentage}%
                  </p>
                  <p className="text-sm text-muted-foreground font-medium mb-1">
                    {sysInfo.battery_percentage < 20
                      ? "Low Battery"
                      : sysInfo.battery_percentage === 100
                        ? "Fully Charged"
                        : "Discharging"}
                  </p>
                </div>
                <div className="w-full bg-muted rounded-full h-2.5 overflow-hidden">
                  <div
                    className={`h-full rounded-full ${sysInfo.battery_percentage < 20 ? "bg-destructive" : "bg-gradient-to-r from-green-400 to-emerald-500"}`}
                    style={{ width: `${sysInfo.battery_percentage}%` }}
                  ></div>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
