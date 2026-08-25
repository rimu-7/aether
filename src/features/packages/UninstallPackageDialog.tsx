import { useState } from "react";
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
import { Package } from "@/types/package";

interface UninstallPackageDialogProps {
  pkg: Package | null;
  onOpenChange: (open: boolean) => void;
  onUninstallComplete: () => void;
}

export function UninstallPackageDialog({ pkg, onOpenChange, onUninstallComplete }: UninstallPackageDialogProps) {
  const [deleting, setDeleting] = useState(false);

  const handleUninstall = async () => {
    if (!pkg) return;
    
    setDeleting(true);
    try {
      await invoke("uninstall_package", { id: pkg.id, isCask: pkg.is_cask });
      toast.success(`${pkg.name} uninstalled successfully.`);
      onUninstallComplete();
      onOpenChange(false);
    } catch (err) {
      toast.error(`Failed to uninstall: ${err}`);
    } finally {
      setDeleting(false);
    }
  };

  return (
    <Dialog open={!!pkg} onOpenChange={(open) => {
      if (!deleting) onOpenChange(open);
    }}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Uninstall {pkg?.name}</DialogTitle>
          <DialogDescription>
            Are you sure you want to completely uninstall this {pkg?.is_cask ? 'application' : 'package'}? This action uses Homebrew and cannot be undone.
          </DialogDescription>
        </DialogHeader>

        <DialogFooter className="mt-4">
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={deleting}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={handleUninstall} disabled={deleting}>
            {deleting ? "Uninstalling..." : "Uninstall"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
