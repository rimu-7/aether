import { Trash2 } from "lucide-react";
import { Application } from "@/types/application";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, useRef } from "react";

interface ApplicationCardProps {
  app: Application;
  onDeleteClick: (app: Application) => void;
}

// Module-level in-memory icon cache to avoid redundant IPC calls
const iconCache = new Map<string, string>();

export function ApplicationCard({ app, onDeleteClick }: ApplicationCardProps) {
  const cacheKey = app.bundle_path || app.id;
  const cachedIcon = iconCache.get(cacheKey) || null;

  const [iconData, setIconData] = useState<string | null>(cachedIcon);
  const [iconFailed, setIconFailed] = useState(false);
  const [isVisible, setIsVisible] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);

  // Lazy icon fetching with IntersectionObserver: only fetch icons when in/near viewport
  useEffect(() => {
    if (cachedIcon) {
      setIconData(cachedIcon);
      return;
    }

    const element = cardRef.current;
    if (!element) return;

    if (!("IntersectionObserver" in window)) {
      setIsVisible(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        const [entry] = entries;
        if (entry.isIntersecting) {
          setIsVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "150px" }
    );

    observer.observe(element);
    return () => observer.disconnect();
  }, [cachedIcon]);

  useEffect(() => {
    if (!isVisible || iconData || iconFailed) return;

    let isMounted = true;
    invoke<string>("get_app_icon", {
      bundlePath: app.bundle_path,
      iconPath: app.icon_path,
    })
      .then((data) => {
        if (!isMounted) return;
        iconCache.set(cacheKey, data);
        setIconData(data);
      })
      .catch(() => {
        if (!isMounted) return;
        setIconFailed(true);
      });

    return () => {
      isMounted = false;
    };
  }, [isVisible, app.bundle_path, app.icon_path, cacheKey, iconData, iconFailed]);

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "Size unavailable";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  return (
    <div
      ref={cardRef}
      className="flex flex-col p-4 border rounded-xl bg-card text-card-foreground shadow-sm group hover:border-primary/40 transition-colors animate-in fade-in duration-200"
    >
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          {iconData && !iconFailed ? (
            <img
              src={iconData}
              alt={app.display_name}
              className="w-12 h-12 rounded-md object-contain select-none transition-opacity duration-200"
              onError={() => setIconFailed(true)}
            />
          ) : (
            <div className="w-12 h-12 bg-muted rounded-md flex items-center justify-center text-xl font-bold text-muted-foreground select-none">
              {app.display_name.charAt(0).toUpperCase()}
            </div>
          )}
          <div className="overflow-hidden">
            <h3
              className="font-semibold leading-none tracking-tight truncate max-w-[180px]"
              title={app.display_name}
            >
              {app.display_name}
            </h3>
            <p className="text-sm text-muted-foreground mt-1 truncate max-w-[180px]">
              {app.version || "Unknown version"}
            </p>
          </div>
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="opacity-0 group-hover:opacity-100 transition-opacity text-destructive hover:text-destructive hover:bg-destructive/10"
          disabled={app.is_system}
          title={
            app.is_system
              ? "System applications cannot be removed"
              : `Remove ${app.display_name}`
          }
          onClick={() => onDeleteClick(app)}
        >
          <Trash2 className="h-4 w-4" />
          <span className="sr-only">Delete {app.display_name}</span>
        </Button>
      </div>

      <div className="mt-4 flex items-center justify-between">
        <div className="flex gap-2">
          {app.is_system && (
            <Badge variant="secondary" className="text-xs">
              System
            </Badge>
          )}
        </div>
        <div className="text-sm font-medium text-muted-foreground">
          {formatBytes(app.size_bytes)}
        </div>
      </div>
    </div>
  );
}
