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
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Application, Artifact } from "@/types/application";
import { DeletionResult } from "@/types/deletion";
import { detectPlatform } from "@/lib/utils";

interface UninstallDialogProps {
  app: Application | null;
  onOpenChange: (open: boolean) => void;
  onUninstallComplete: (app: Application) => void;
}

export function UninstallDialog({ app, onOpenChange, onUninstallComplete }: UninstallDialogProps) {
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [loading, setLoading] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const isWindowsRegistryApp = Boolean(
    app
      && detectPlatform() === "windows"
      && /^(HKLM64|HKLM32|HKCU):/.test(app.id)
  );

  useEffect(() => {
    setConfirmOpen(false);
    if (!app) return;
    
    setLoading(true);
    setArtifacts([]);
    setSelectedPaths(new Set());
    
    invoke<Artifact[]>("get_application_artifacts", { 
      bundleId: app.bundle_id, 
      appName: app.name,
      bundlePath: app.bundle_path,
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
      const shouldRunWindowsUninstaller = isWindowsRegistryApp && selectedPaths.has(app.bundle_path);
      if (shouldRunWindowsUninstaller) {
        await invoke("uninstall_package", {
          id: app.id,
          isCask: true,
          manager: "windows-registry",
        });
      }

      // The Windows installer owns its Program Files folder; do not try to
      // move that folder separately after its native uninstaller has run.
      const artifactPaths = shouldRunWindowsUninstaller
        ? pathsToDelete.filter((path) => path !== app.bundle_path)
        : pathsToDelete;
      const result: DeletionResult = artifactPaths.length > 0
        ? await invoke("delete_artifacts", { paths: artifactPaths })
        : { deleted: [], failures: [] };
      const applicationWasRemoved = shouldRunWindowsUninstaller || result.deleted.includes(app.bundle_path);
      
      if (result.failures.length > 0) {
        toast.warning(`Moved ${result.deleted.length} of ${pathsToDelete.length} selected items to Trash. ${result.failures.length} could not be moved.`);
      } else if (shouldRunWindowsUninstaller) {
        toast.success(`${app.display_name} was uninstalled. Selected leftover data was moved to Trash.`);
      } else if (applicationWasRemoved) {
        toast.success(`Moved ${app.display_name} and its selected data to Trash.`);
      } else {
        toast.success("Moved selected application data to Trash.");
      }
      
      setConfirmOpen(false);
      if (applicationWasRemoved) {
        onUninstallComplete(app);
      }
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

  const selectedApplicationSize = app && selectedPaths.has(app.bundle_path) ? app.size_bytes : 0;
  const selectedArtifactSize = artifacts
    .filter(a => selectedPaths.has(a.path))
    .reduce((sum, a) => sum + a.size_bytes, 0);
  const totalSelectedSize = selectedApplicationSize + selectedArtifactSize;

  return (
    <>
      <Dialog open={!!app} onOpenChange={(open) => {
        if (!deleting && !confirmOpen) onOpenChange(open);
      }}>
      <DialogContent
        className="sm:max-w-[600px] max-h-[85vh] flex flex-col"
        showCloseButton={!deleting && !confirmOpen}
      >
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
                    No related application data found on this system.
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
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={deleting || confirmOpen}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={() => setConfirmOpen(true)}
            disabled={deleting || selectedPaths.size === 0 || loading}
          >
            Uninstall Selected
          </Button>
        </DialogFooter>
      </DialogContent>
      </Dialog>

      <AlertDialog open={confirmOpen} onOpenChange={(open) => {
        if (!deleting) setConfirmOpen(open);
      }}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete selected application data?</AlertDialogTitle>
            <AlertDialogDescription>
              {isWindowsRegistryApp && selectedPaths.has(app?.bundle_path || "")
                ? `Windows will run this application's registered uninstaller${selectedPaths.size > 1 ? " and move selected leftover data to Trash" : ""}.`
                : `This will move ${selectedPaths.size} selected ${selectedPaths.size === 1 ? "item" : "items"} (${formatBytes(totalSelectedSize)}) to your system Trash.`}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deleting}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              disabled={deleting || selectedPaths.size === 0}
              onClick={handleUninstall}
            >
              {deleting ? "Uninstalling..." : isWindowsRegistryApp && selectedPaths.has(app?.bundle_path || "") ? "Uninstall application" : "Move selected items to Trash"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
