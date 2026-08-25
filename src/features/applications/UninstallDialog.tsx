import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Application, Artifact } from "@/types/application";

interface UninstallDialogProps {
  app: Application | null;
  onOpenChange: (open: boolean) => void;
  onUninstallComplete: () => void;
}

export function UninstallDialog({ app, onOpenChange, onUninstallComplete }: UninstallDialogProps) {
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [loading, setLoading] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (!app) return;
    
    setLoading(true);
    setArtifacts([]);
    setSelectedPaths(new Set());
    
    invoke<Artifact[]>("get_application_artifacts", { 
      bundleId: app.bundle_id, 
      appName: app.name 
    })
      .then((data) => {
        setArtifacts(data);
        // By default, only select EXACT and HIGH confidence items + the app bundle itself
        const safePaths = new Set<string>();
        safePaths.add(app.bundle_path); // Always include the app itself
        
        data.forEach(a => {
          if (a.confidence === "Exact" || a.confidence === "High") {
            safePaths.add(a.path);
          }
        });
        
        setSelectedPaths(safePaths);
      })
      .catch((err) => {
        toast.error(`Failed to scan artifacts: ${err}`);
      })
      .finally(() => {
        setLoading(false);
      });
  }, [app]);

  const handleTogglePath = (path: string) => {
    const next = new Set(selectedPaths);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    setSelectedPaths(next);
  };

  const handleUninstall = async () => {
    if (!app || selectedPaths.size === 0) return;
    
    setDeleting(true);
    try {
      const pathsToDelete = Array.from(selectedPaths);
      const deleted: string[] = await invoke("delete_artifacts", { paths: pathsToDelete });
      
      if (deleted.length < pathsToDelete.length) {
        toast.warning(`Uninstalled with warnings. Removed ${deleted.length} of ${pathsToDelete.length} items.`);
      } else {
        toast.success(`${app.display_name} uninstalled successfully.`);
      }
      
      onUninstallComplete();
      onOpenChange(false);
    } catch (err) {
      toast.error(`Failed to uninstall: ${err}`);
    } finally {
      setDeleting(false);
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  const totalSelectedSize = app?.bundle_path && selectedPaths.has(app.bundle_path) ? app.size_bytes : 0 
    + artifacts.filter(a => selectedPaths.has(a.path)).reduce((sum, a) => sum + a.size_bytes, 0);

  return (
    <Dialog open={!!app} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[600px] max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Uninstall {app?.display_name}</DialogTitle>
          <DialogDescription>
            Review the associated application data before permanently deleting it.
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-hidden py-4 flex flex-col gap-4">
          {loading ? (
            <div className="space-y-4">
              <Skeleton className="h-4 w-3/4" />
              <Skeleton className="h-4 w-1/2" />
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-20 w-full" />
            </div>
          ) : (
            <ScrollArea className="h-[400px] rounded-md border p-4">
              <div className="space-y-4">
                {/* Always show the main application bundle */}
                <div className="flex items-start space-x-3">
                  <Checkbox 
                    id="app-bundle" 
                    checked={selectedPaths.has(app?.bundle_path || "")} 
                    onCheckedChange={() => app && handleTogglePath(app.bundle_path)}
                  />
                  <div className="flex-1 space-y-1 leading-none">
                    <label htmlFor="app-bundle" className="text-sm font-medium leading-none cursor-pointer">
                      Application Bundle
                    </label>
                    <p className="text-sm text-muted-foreground break-all">{app?.bundle_path}</p>
                  </div>
                  <div className="text-sm font-medium">{formatBytes(app?.size_bytes || 0)}</div>
                </div>

                {artifacts.length > 0 && <div className="h-px bg-border my-4" />}

                {/* Show related artifacts */}
                {artifacts.map((a, i) => (
                  <div key={i} className="flex items-start space-x-3">
                    <Checkbox 
                      id={`artifact-${i}`} 
                      checked={selectedPaths.has(a.path)} 
                      onCheckedChange={() => handleTogglePath(a.path)}
                    />
                    <div className="flex-1 space-y-1 leading-none">
                      <div className="flex items-center gap-2">
                        <label htmlFor={`artifact-${i}`} className="text-sm font-medium leading-none cursor-pointer">
                          {a.category}
                        </label>
                        <Badge variant="outline" className="text-[10px] py-0 h-4">
                          {a.confidence}
                        </Badge>
                      </div>
                      <p className="text-sm text-muted-foreground break-all">{a.path}</p>
                    </div>
                    <div className="text-sm font-medium">{formatBytes(a.size_bytes)}</div>
                  </div>
                ))}

                {artifacts.length === 0 && (
                  <p className="text-sm text-muted-foreground text-center py-8">
                    No related application data found on this Mac.
                  </p>
                )}
              </div>
            </ScrollArea>
          )}

          <div className="flex justify-between items-center text-sm">
            <span className="text-muted-foreground">Estimated space to recover:</span>
            <span className="font-bold text-lg">{formatBytes(totalSelectedSize)}</span>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={deleting}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={handleUninstall} disabled={deleting || selectedPaths.size === 0 || loading}>
            {deleting ? "Uninstalling..." : "Uninstall Selected"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
