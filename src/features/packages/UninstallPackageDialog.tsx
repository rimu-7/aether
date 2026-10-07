import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
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
import { Package } from "@/types/package";

interface UninstallPackageDialogProps {
  pkg: Package | null;
  onOpenChange: (open: boolean) => void;
  onUninstallComplete: (pkg: Package) => void;
}

export function UninstallPackageDialog({ pkg, onOpenChange, onUninstallComplete }: UninstallPackageDialogProps) {
  const [deleting, setDeleting] = useState(false);

  const handleUninstall = async () => {
    if (!pkg) return;
    
    setDeleting(true);
    try {
      await invoke("uninstall_package", {
        id: pkg.id,
        isCask: pkg.is_cask,
        manager: pkg.manager,
      });
      toast.success(`${pkg.name} uninstalled successfully.`);
      onUninstallComplete(pkg);
      onOpenChange(false);
    } catch (err) {
      toast.error(`Failed to uninstall: ${err}`);
    } finally {
      setDeleting(false);
    }
  };

  return (
    <AlertDialog open={!!pkg} onOpenChange={(open) => {
      if (!deleting) onOpenChange(open);
    }}>
      <AlertDialogContent className="sm:max-w-[425px]">
        <AlertDialogHeader>
          <AlertDialogTitle>Uninstall {pkg?.name}</AlertDialogTitle>
          <AlertDialogDescription>
            Are you sure you want to completely uninstall this {pkg?.is_cask ? 'application' : 'package'}? This action cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>

        <AlertDialogFooter className="mt-4">
          <AlertDialogCancel disabled={deleting}>
            Cancel
          </AlertDialogCancel>
          <AlertDialogAction variant="destructive" onClick={handleUninstall} disabled={deleting}>
            {deleting ? "Uninstalling..." : "Uninstall"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
