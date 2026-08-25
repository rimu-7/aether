import { Trash2 } from "lucide-react";
import { Application } from "@/types/application";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

interface ApplicationCardProps {
  app: Application;
  onDeleteClick: (app: Application) => void;
}

export function ApplicationCard({ app, onDeleteClick }: ApplicationCardProps) {
  const [iconData, setIconData] = useState<string | null>(null);
  const [iconFailed, setIconFailed] = useState(false);

  useEffect(() => {
    invoke<string>("get_app_icon", { bundlePath: app.bundle_path })
      .then(setIconData)
      .catch(() => setIconFailed(true));
  }, [app.bundle_path]);

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  return (
    <div className="flex flex-col p-4 border rounded-xl bg-card text-card-foreground shadow-sm group">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          {iconData && !iconFailed ? (
            <img src={iconData} alt={app.display_name} className="w-12 h-12 rounded-md object-contain select-none" />
          ) : (
            <div className="w-12 h-12 bg-muted rounded-md flex items-center justify-center text-xl font-bold text-muted-foreground select-none">
              {app.display_name.charAt(0)}
            </div>
          )}
          <div>
            <h3 className="font-semibold leading-none tracking-tight truncate max-w-[200px]" title={app.display_name}>
              {app.display_name}
            </h3>
            <p className="text-sm text-muted-foreground mt-1">
              {app.version || "Unknown version"}
            </p>
          </div>
        </div>
        <Button 
          variant="ghost" 
          size="icon" 
          className="opacity-0 group-hover:opacity-100 transition-opacity text-destructive hover:text-destructive hover:bg-destructive/10"
          onClick={() => onDeleteClick(app)}
        >
          <Trash2 className="h-4 w-4" />
          <span className="sr-only">Delete {app.display_name}</span>
        </Button>
      </div>
      
      <div className="mt-4 flex items-center justify-between">
        <div className="flex gap-2">
          {app.is_system && (
            <Badge variant="secondary" className="text-xs">System</Badge>
          )}
        </div>
        <div className="text-sm font-medium">
          {formatBytes(app.size_bytes)}
        </div>
      </div>
    </div>
  );
}
