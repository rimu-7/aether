import { Trash2, Terminal, AppWindow } from "lucide-react";
import { Package } from "@/types/package";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

interface PackageCardProps {
  pkg: Package;
  onDeleteClick: (pkg: Package) => void;
}

export function PackageCard({ pkg, onDeleteClick }: PackageCardProps) {
  return (
    <div className="flex flex-col p-4 border rounded-xl bg-card text-card-foreground shadow-sm group">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-muted rounded-md flex items-center justify-center text-muted-foreground select-none shrink-0">
            {pkg.is_cask ? <AppWindow className="h-5 w-5" /> : <Terminal className="h-5 w-5" />}
          </div>
          <div className="overflow-hidden">
            <h3 className="font-semibold leading-none tracking-tight truncate max-w-[200px]" title={pkg.name}>
              {pkg.name}
            </h3>
            <p className="text-sm text-muted-foreground mt-1">
              v{pkg.version}
            </p>
          </div>
        </div>
        <Button 
          variant="ghost" 
          size="icon" 
          className="opacity-0 group-hover:opacity-100 transition-opacity text-destructive hover:text-destructive hover:bg-destructive/10 shrink-0"
          onClick={() => onDeleteClick(pkg)}
        >
          <Trash2 className="h-4 w-4" />
          <span className="sr-only">Delete {pkg.name}</span>
        </Button>
      </div>
      
      {pkg.description && (
        <p className="text-sm text-muted-foreground mt-3 line-clamp-2" title={pkg.description}>
          {pkg.description}
        </p>
      )}
      
      <div className="mt-auto pt-4 flex items-center justify-between">
        <div className="flex gap-2">
          {pkg.is_cask ? (
            <Badge variant="secondary" className="text-xs bg-blue-500/10 text-blue-500 hover:bg-blue-500/20">Cask</Badge>
          ) : (
            <Badge variant="secondary" className="text-xs bg-orange-500/10 text-orange-500 hover:bg-orange-500/20">Formula</Badge>
          )}
        </div>
      </div>
    </div>
  );
}
