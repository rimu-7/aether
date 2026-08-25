import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Package } from "@/types/package";
import { PackageCard } from "./PackageCard";
import { UninstallPackageDialog } from "./UninstallPackageDialog";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

export function Packages() {
  const [packages, setPackages] = useState<Package[]>([]);
  const [filteredPackages, setFilteredPackages] = useState<Package[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<"all" | "formulae" | "casks">("all");
  
  const [selectedPkg, setSelectedPkg] = useState<Package | null>(null);

  const fetchPackages = () => {
    setLoading(true);
    invoke<Package[]>("get_installed_packages")
      .then((data) => {
        data.sort((a, b) => a.name.localeCompare(b.name));
        setPackages(data);
        setFilteredPackages(data);
      })
      .catch((err) => {
        toast.error(`Failed to load packages: ${err}`);
      })
      .finally(() => {
        setLoading(false);
      });
  };

  useEffect(() => {
    fetchPackages();
  }, []);

  useEffect(() => {
    const term = search.toLowerCase();
    setFilteredPackages(
      packages.filter(pkg => {
        const matchesSearch = pkg.name.toLowerCase().includes(term) || pkg.description?.toLowerCase().includes(term);
        const matchesType = filter === "all" || (filter === "casks" && pkg.is_cask) || (filter === "formulae" && !pkg.is_cask);
        return matchesSearch && matchesType;
      })
    );
  }, [search, filter, packages]);

  return (
    <div className="flex flex-col gap-6 h-full min-h-0">
      <div className="flex items-center justify-between shrink-0">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Packages</h1>
          <p className="text-muted-foreground">Manage your Homebrew formulae and casks.</p>
        </div>
        <div className="flex gap-2">
          <div className="flex rounded-md border bg-muted p-1">
            <Button 
              variant={filter === "all" ? "secondary" : "ghost"} 
              size="sm" 
              className="h-7 px-3 text-xs" 
              onClick={() => setFilter("all")}
            >
              All
            </Button>
            <Button 
              variant={filter === "casks" ? "secondary" : "ghost"} 
              size="sm" 
              className="h-7 px-3 text-xs" 
              onClick={() => setFilter("casks")}
            >
              Casks
            </Button>
            <Button 
              variant={filter === "formulae" ? "secondary" : "ghost"} 
              size="sm" 
              className="h-7 px-3 text-xs" 
              onClick={() => setFilter("formulae")}
            >
              Formulae
            </Button>
          </div>
          <div className="w-64">
            <Input 
              placeholder="Search packages..." 
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto min-h-0 pr-2">
        {loading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {Array.from({ length: 9 }).map((_, i) => (
              <div key={i} className="flex flex-col p-4 border rounded-xl space-y-3">
                <div className="flex items-center gap-3">
                  <Skeleton className="w-10 h-10 rounded-md shrink-0" />
                  <div className="space-y-2 w-full">
                    <Skeleton className="h-4 w-[120px]" />
                    <Skeleton className="h-3 w-[60px]" />
                  </div>
                </div>
                <Skeleton className="h-8 w-full mt-2" />
                <div className="flex items-center justify-between mt-auto pt-2">
                  <Skeleton className="h-5 w-[60px] rounded-full" />
                </div>
              </div>
            ))}
          </div>
        ) : filteredPackages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <h3 className="text-lg font-medium">No packages found</h3>
            <p className="text-muted-foreground mt-1">Make sure Homebrew is installed or try a different search.</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 pb-12">
            {filteredPackages.map((pkg) => (
              <PackageCard 
                key={pkg.id} 
                pkg={pkg} 
                onDeleteClick={setSelectedPkg} 
              />
            ))}
          </div>
        )}
      </div>

      <UninstallPackageDialog 
        pkg={selectedPkg} 
        onOpenChange={(open) => !open && setSelectedPkg(null)}
        onUninstallComplete={fetchPackages}
      />
    </div>
  );
}
