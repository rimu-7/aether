import { useState, useEffect, useMemo } from "react";
import { toast } from "sonner";
import { ArrowUpDown, RefreshCw } from "lucide-react";
import { Application } from "@/types/application";
import { ApplicationCard } from "./ApplicationCard";
import { UninstallDialog } from "./UninstallDialog";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { PageHeader } from "@/components/layout/PageHeader";
import { useScanStore } from "@/stores/scanStore";
import { useProgressiveList } from "@/hooks/useProgressiveList";

type SortOption = "name-asc" | "name-desc" | "size-desc" | "size-asc";
type TypeFilter = "all" | "user" | "system";

export function Applications() {
  const applications = useScanStore((state) => state.applications);
  const status = useScanStore((state) => state.applicationsStatus);
  const scanError = useScanStore((state) => state.applicationsError);
  const ensureApplicationsLoaded = useScanStore((state) => state.ensureApplicationsLoaded);
  const refreshApplications = useScanStore((state) => state.refreshApplications);
  const removeApplication = useScanStore((state) => state.removeApplication);

  const [search, setSearch] = useState("");
  const [typeFilter, setTypeFilter] = useState<TypeFilter>("all");
  const [sortBy, setSortBy] = useState<SortOption>("name-asc");
  const [selectedApp, setSelectedApp] = useState<Application | null>(null);

  useEffect(() => {
    void ensureApplicationsLoaded();
  }, [ensureApplicationsLoaded]);

  const loading = status === "idle" || status === "loading";

  const filteredAndSortedApps = useMemo(() => {
    const term = search.toLowerCase().trim();
    let result = applications.filter((app) => {
      const matchesSearch =
        !term ||
        app.display_name.toLowerCase().includes(term) ||
        app.bundle_id?.toLowerCase().includes(term);

      const matchesType =
        typeFilter === "all" ||
        (typeFilter === "system" && app.is_system) ||
        (typeFilter === "user" && !app.is_system);

      return matchesSearch && matchesType;
    });

    return result.sort((a, b) => {
      if (sortBy === "name-asc") {
        return a.display_name.localeCompare(b.display_name);
      }
      if (sortBy === "name-desc") {
        return b.display_name.localeCompare(a.display_name);
      }
      if (sortBy === "size-desc") {
        return b.size_bytes - a.size_bytes;
      }
      if (sortBy === "size-asc") {
        return a.size_bytes - b.size_bytes;
      }
      return 0;
    });
  }, [applications, search, typeFilter, sortBy]);

  const { visibleItems, visibleCount, totalCount, isStreaming } =
    useProgressiveList(filteredAndSortedApps, {
      initialBatchSize: 12,
      streamBatchSize: 6,
      tickMs: 25,
      resetKey: `${search}-${typeFilter}-${sortBy}`,
    });

  const handleRefresh = async () => {
    try {
      await refreshApplications();
      toast.success("Applications scan refreshed");
    } catch (err) {
      toast.error(`Failed to refresh: ${err}`);
    }
  };

  return (
    <div className="flex flex-col gap-6 h-full min-h-0">
      {/* Standardized Reusable Page Header */}
      <PageHeader
        title="Applications"
        description="Manage and uninstall applications installed on your system."
        actions={
          <Button
            variant="outline"
            size="sm"
            className="h-9 gap-1.5 text-xs font-medium"
            onClick={handleRefresh}
            disabled={loading}
            title="Rescan applications"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
            Refresh
          </Button>
        }
        filters={
          <div className="flex rounded-md border bg-muted/60 p-1">
            <Button
              variant={typeFilter === "all" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-3 text-xs"
              onClick={() => setTypeFilter("all")}
            >
              All
            </Button>
            <Button
              variant={typeFilter === "user" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-3 text-xs"
              onClick={() => setTypeFilter("user")}
            >
              User
            </Button>
            <Button
              variant={typeFilter === "system" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-3 text-xs"
              onClick={() => setTypeFilter("system")}
            >
              System
            </Button>
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
          placeholder: "Search applications...",
        }}
      />

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto min-h-0 pr-2">
        {loading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {Array.from({ length: 12 }).map((_, i) => (
              <div
                key={i}
                className="flex flex-col p-4 border rounded-xl space-y-3 bg-card shadow-sm animate-pulse"
              >
                <div className="flex items-center gap-3">
                  <Skeleton className="w-12 h-12 rounded-md shrink-0" />
                  <div className="space-y-2 w-full">
                    <Skeleton className="h-4 w-[140px]" />
                    <Skeleton className="h-3 w-[90px]" />
                  </div>
                </div>
                <div className="flex items-center justify-between mt-4 pt-1">
                  <Skeleton className="h-5 w-[60px] rounded-full" />
                  <Skeleton className="h-4 w-[50px]" />
                </div>
              </div>
            ))}
          </div>
        ) : scanError ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <h3 className="text-lg font-medium">Couldn&apos;t scan applications</h3>
            <p className="max-w-xl text-muted-foreground mt-1">{scanError}</p>
            <Button variant="outline" className="mt-4" onClick={handleRefresh}>
              Try again
            </Button>
          </div>
        ) : filteredAndSortedApps.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <h3 className="text-lg font-medium">No applications found</h3>
            <p className="text-muted-foreground mt-1">
              Try adjusting your search query or filter.
            </p>
          </div>
        ) : (
          <div>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 pb-6">
              {visibleItems.map((app) => (
                <ApplicationCard
                  key={app.id}
                  app={app}
                  onDeleteClick={setSelectedApp}
                />
              ))}
            </div>

            {isStreaming && (
              <div className="flex items-center justify-center py-4 text-xs text-muted-foreground gap-2">
                <span className="inline-block h-2 w-2 rounded-full bg-primary/60 animate-ping" />
                Loading remaining applications ({visibleCount} of {totalCount})...
              </div>
            )}
          </div>
        )}
      </div>

      <UninstallDialog
        app={selectedApp}
        onOpenChange={(open) => !open && setSelectedApp(null)}
        onUninstallComplete={(removedApp) => removeApplication(removedApp.id)}
      />
    </div>
  );
}
