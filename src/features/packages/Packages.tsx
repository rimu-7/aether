import { useState, useEffect, useMemo } from "react";
import { toast } from "sonner";
import { ArrowUpDown, RefreshCw } from "lucide-react";
import { Package } from "@/types/package";
import { PackageCard } from "./PackageCard";
import { UninstallPackageDialog } from "./UninstallPackageDialog";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { detectPlatform } from "@/lib/utils";
import { PageHeader } from "@/components/layout/PageHeader";
import { useScanStore } from "@/stores/scanStore";
import { useProgressiveList } from "@/hooks/useProgressiveList";

type SortOption = "name-asc" | "name-desc" | "size-desc" | "size-asc";

export function Packages() {
  const packages = useScanStore((state) => state.packages);
  const status = useScanStore((state) => state.packagesStatus);
  const loadError = useScanStore((state) => state.packagesError);
  const ensurePackagesLoaded = useScanStore((state) => state.ensurePackagesLoaded);
  const refreshPackages = useScanStore((state) => state.refreshPackages);
  const removePackage = useScanStore((state) => state.removePackage);

  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<"all" | "formulae" | "casks">("all");
  const [sortBy, setSortBy] = useState<SortOption>("name-asc");
  const [selectedPkg, setSelectedPkg] = useState<Package | null>(null);

  const platform = detectPlatform();

  useEffect(() => {
    void ensurePackagesLoaded();
  }, [ensurePackagesLoaded]);

  const loading = status === "idle" || status === "loading";

  const handleRefresh = async () => {
    try {
      await refreshPackages();
      toast.success("Packages refreshed");
    } catch (err) {
      toast.error(`Failed to refresh packages: ${err}`);
    }
  };

  const filteredAndSortedPackages = useMemo(() => {
    const term = search.toLowerCase().trim();
    let result = packages.filter((pkg) => {
      const matchesSearch =
        !term ||
        pkg.name.toLowerCase().includes(term) ||
        pkg.description?.toLowerCase().includes(term);

      const matchesType =
        filter === "all" ||
        (filter === "casks" && pkg.is_cask) ||
        (filter === "formulae" && !pkg.is_cask);

      return matchesSearch && matchesType;
    });

    return result.sort((a, b) => {
      if (sortBy === "name-asc") {
        return a.name.localeCompare(b.name);
      }
      if (sortBy === "name-desc") {
        return b.name.localeCompare(a.name);
      }
      if (sortBy === "size-desc") {
        return b.size_bytes - a.size_bytes;
      }
      if (sortBy === "size-asc") {
        return a.size_bytes - b.size_bytes;
      }
      return 0;
    });
  }, [packages, search, filter, sortBy]);

  const { visibleItems, visibleCount, totalCount, isStreaming } =
    useProgressiveList(filteredAndSortedPackages, {
      initialBatchSize: 12,
      streamBatchSize: 6,
      tickMs: 25,
      resetKey: `${search}-${filter}-${sortBy}`,
    });

  return (
    <div className="flex flex-col gap-6 h-full min-h-0">
      {/* Standardized Reusable Page Header */}
      <PageHeader
        title="Packages"
        description="Manage your installed system packages."
        actions={
          <Button
            variant="outline"
            size="sm"
            className="h-9 gap-1.5 text-xs font-medium"
            onClick={handleRefresh}
            disabled={loading}
            title="Rescan packages"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
            Refresh
          </Button>
        }
        filters={
          <div className="flex rounded-md border bg-muted/60 p-1">
            <Button
              variant={filter === "all" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-3 text-xs"
              onClick={() => setFilter("all")}
            >
              All
            </Button>
            {platform === "macos" ? (
              <>
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
              </>
            ) : (
              <>
                <Button
                  variant={filter === "casks" ? "secondary" : "ghost"}
                  size="sm"
                  className="h-7 px-3 text-xs"
                  onClick={() => setFilter("casks")}
                >
                  Applications
                </Button>
                <Button
                  variant={filter === "formulae" ? "secondary" : "ghost"}
                  size="sm"
                  className="h-7 px-3 text-xs"
                  onClick={() => setFilter("formulae")}
                >
                  System
                </Button>
              </>
            )}
          </div>
        }
        sort={
          <div className="flex rounded-md border bg-muted/60 p-1">
            <Button
              variant={sortBy === "name-asc" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-2.5 text-xs font-medium"
              onClick={() => setSortBy("name-asc")}
              title="Sort Alphabetically A to Z"
            >
              A → Z
            </Button>
            <Button
              variant={sortBy === "name-desc" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-2.5 text-xs font-medium"
              onClick={() => setSortBy("name-desc")}
              title="Sort Alphabetically Z to A"
            >
              Z → A
            </Button>
            <Button
              variant={sortBy === "size-desc" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-2.5 text-xs font-medium"
              onClick={() => setSortBy("size-desc")}
              title="Sort by Size (Largest first)"
            >
              Size <ArrowUpDown className="ml-1 h-3 w-3 inline" />
            </Button>
          </div>
        }
        search={{
          value: search,
          onChange: setSearch,
          placeholder: "Search packages...",
        }}
      />

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto min-h-0 pr-2">
        {loading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {Array.from({ length: 9 }).map((_, i) => (
              <div
                key={i}
                className="flex flex-col p-4 border rounded-xl space-y-3 bg-card shadow-sm animate-pulse"
              >
                <div className="flex items-center gap-3">
                  <Skeleton className="w-10 h-10 rounded-md shrink-0" />
                  <div className="space-y-2 w-full">
                    <Skeleton className="h-4 w-[130px]" />
                    <Skeleton className="h-3 w-[70px]" />
                  </div>
                </div>
                <Skeleton className="h-8 w-full mt-2" />
                <div className="flex items-center justify-between mt-auto pt-2">
                  <Skeleton className="h-5 w-[60px] rounded-full" />
                  <Skeleton className="h-4 w-[50px]" />
                </div>
              </div>
            ))}
          </div>
        ) : loadError ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <h3 className="text-lg font-medium">Couldn&apos;t load packages</h3>
            <p className="max-w-xl text-muted-foreground mt-1">{loadError}</p>
            <Button variant="outline" className="mt-4" onClick={handleRefresh}>
              Try again
            </Button>
          </div>
        ) : filteredAndSortedPackages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <h3 className="text-lg font-medium">No packages found</h3>
            <p className="text-muted-foreground mt-1">
              No packages were found matching your query or filter.
            </p>
          </div>
        ) : (
          <div>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 pb-6">
              {visibleItems.map((pkg) => (
                <PackageCard
                  key={pkg.id}
                  pkg={pkg}
                  onDeleteClick={setSelectedPkg}
                />
              ))}
            </div>

            {isStreaming && (
              <div className="flex items-center justify-center py-4 text-xs text-muted-foreground gap-2">
                <span className="inline-block h-2 w-2 rounded-full bg-primary/60 animate-ping" />
                Loading remaining packages ({visibleCount} of {totalCount})...
              </div>
            )}
          </div>
        )}
      </div>

      <UninstallPackageDialog
        pkg={selectedPkg}
        onOpenChange={(open) => !open && setSelectedPkg(null)}
        onUninstallComplete={(removedPkg) => removePackage(removedPkg.id)}
      />
    </div>
  );
}
