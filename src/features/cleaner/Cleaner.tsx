import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { DeletionResult } from "@/types/deletion";
import { FolderSearch, ArrowUpDown, ArrowUp, ArrowDown, RefreshCw } from "lucide-react";
import { useScanStore } from "@/stores/scanStore";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { useProgressiveList } from "@/hooks/useProgressiveList";
import { PageHeader } from "@/components/layout/PageHeader";
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

export function Cleaner() {
  const items = useScanStore((state) => state.cleanableItems);
  const scanStatus = useScanStore((state) => state.cleanerStatus);
  const scanError = useScanStore((state) => state.cleanerError);
  const ensureCleanerLoaded = useScanStore((state) => state.ensureCleanerLoaded);
  const refreshCleaner = useScanStore((state) => state.refreshCleaner);
  const removeCleanablePaths = useScanStore((state) => state.removeCleanablePaths);
  const [search, setSearch] = useState("");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [deleting, setDeleting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [sortBy, setSortBy] = useState<"size" | "name">("size");
  const [sortDesc, setSortDesc] = useState(true);

  useEffect(() => {
    // Fallback for this route being mounted outside the normal app shell.
    // In the normal flow AppLayout starts this scan before navigation.
    void ensureCleanerLoaded();
  }, [ensureCleanerLoaded]);

  useEffect(() => {
    const availableIds = new Set(items.map((item) => item.id));
    setSelectedIds((current) => {
      const next = new Set([...current].filter((id) => availableIds.has(id)));
      return next.size === current.size ? current : next;
    });
  }, [items]);

  const loading = scanStatus === "idle" || scanStatus === "loading";

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  const filteredAndSortedItems = useMemo(() => {
    const term = search.toLowerCase();
    const filtered = items.filter(item => 
      item.name.toLowerCase().includes(term) || 
      item.item_type.toLowerCase().includes(term)
    );

    return filtered.sort((a, b) => {
      let comparison = 0;
      if (sortBy === "size") {
        comparison = a.size_bytes - b.size_bytes;
      } else {
        comparison = a.name.localeCompare(b.name);
      }
      return sortDesc ? -comparison : comparison;
    });
  }, [items, search, sortBy, sortDesc]);

  const { visibleItems, visibleCount, totalCount, isStreaming, loadMore } =
    useProgressiveList(filteredAndSortedItems, {
      initialBatchSize: 30,
      streamBatchSize: 25,
      tickMs: 25,
      resetKey: `${search}-${sortBy}-${sortDesc}`,
    });

  const toggleSelection = (id: string) => {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelectedIds(next);
  };

  const toggleAll = () => {
    if (selectedIds.size === filteredAndSortedItems.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredAndSortedItems.map(i => i.id)));
    }
  };

  const handleReveal = async (path: string) => {
    try {
      await invoke("reveal_in_finder", { path });
    } catch (err) {
      toast.error(`Failed to reveal file: ${err}`);
    }
  };

  const handleDeleteSelected = async () => {
    if (selectedIds.size === 0) return;
    
    const pathsToDelete = items
      .filter(i => selectedIds.has(i.id))
      .map(i => i.absolute_path);
      
    setDeleting(true);
    try {
      const result = await invoke<DeletionResult>("delete_cleanable_items", { paths: pathsToDelete });
      if (result.deleted.length > 0) {
        toast.success(`Moved ${result.deleted.length} ${result.deleted.length === 1 ? "item" : "items"} to Trash.`);
      }
      if (result.failures.length > 0) {
        toast.warning(`${result.failures.length} ${result.failures.length === 1 ? "item could" : "items could"} not be moved: ${result.failures[0].reason}`);
      }
      setSelectedIds(new Set());
      setConfirmOpen(false);
      removeCleanablePaths(result.deleted);
    } catch (err) {
      toast.error(`Failed to clean items: ${err}`);
    } finally {
      setDeleting(false);
    }
  };

  const totalSelectedSize = useMemo(() => {
    return items
      .filter(i => selectedIds.has(i.id))
      .reduce((sum, item) => sum + item.size_bytes, 0);
  }, [items, selectedIds]);

  const toggleSort = (field: "size" | "name") => {
    if (sortBy === field) {
      setSortDesc(!sortDesc);
    } else {
      setSortBy(field);
      setSortDesc(true); // default desc for size, asc for name usually, but let's just do true
    }
  };

  return (
    <div className="flex flex-col gap-6 h-full min-h-0">
      {/* Standardized Reusable Page Header */}
      <PageHeader
        title="Deep Cleaner"
        description="Reclaim disk space by safely removing caches and temporary files."
        actions={
          <>
            {selectedIds.size > 0 && (
              <Button
                variant="destructive"
                size="sm"
                className="h-9 gap-1.5 text-xs font-medium shrink-0"
                disabled={deleting}
                onClick={() => setConfirmOpen(true)}
              >
                {deleting ? "Cleaning..." : `Clean Selected (${formatBytes(totalSelectedSize)})`}
              </Button>
            )}
            <Button
              variant="outline"
              size="sm"
              className="h-9 gap-1.5 text-xs font-medium shrink-0"
              onClick={() => void refreshCleaner()}
              disabled={loading || deleting}
              title="Rescan cleanable items"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
              Refresh
            </Button>
          </>
        }
        filters={
          selectedIds.size > 0 ? (
            <span className="text-xs text-muted-foreground font-medium px-1 truncate">
              {selectedIds.size} of {filteredAndSortedItems.length} selected
            </span>
          ) : (
            <span className="text-xs text-muted-foreground px-1 truncate">
              {filteredAndSortedItems.length} items found
            </span>
          )
        }
        sort={
          <div className="flex rounded-md border bg-muted/60 p-0.5 shrink-0">
            <Button
              variant={sortBy === "size" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-2.5 text-xs font-medium"
              onClick={() => toggleSort("size")}
              title="Sort by size"
            >
              Size {sortBy === "size" ? (sortDesc ? "↓" : "↑") : ""}
            </Button>
            <Button
              variant={sortBy === "name" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-2.5 text-xs font-medium"
              onClick={() => toggleSort("name")}
              title="Sort by name"
            >
              Name {sortBy === "name" ? (sortDesc ? "↓" : "↑") : ""}
            </Button>
          </div>
        }
        search={{
          value: search,
          onChange: setSearch,
          placeholder: "Search caches...",
        }}
      />

      <div className="flex-1 border rounded-md overflow-hidden flex flex-col min-h-0 bg-card">
        <div
          className="flex-1 overflow-y-auto overflow-x-auto min-h-0"
          onScroll={(e) => {
            const el = e.currentTarget;
            if (el.scrollHeight - el.scrollTop - el.clientHeight < 400) {
              loadMore();
            }
          }}
        >
          {/* Sticky Table Header */}
          <div className="sticky top-0 z-20 flex items-center gap-3 px-4 py-2.5 border-b bg-muted/95 backdrop-blur-md font-medium text-xs sm:text-sm text-muted-foreground select-none min-w-[500px]">
            <div className="w-9 shrink-0 flex items-center justify-center">
              <Checkbox 
                checked={filteredAndSortedItems.length > 0 && selectedIds.size === filteredAndSortedItems.length}
                onCheckedChange={toggleAll}
                aria-label="Select all items"
              />
            </div>
            <div 
              className="flex-1 min-w-[160px] flex items-center cursor-pointer hover:text-foreground transition-colors overflow-hidden" 
              onClick={() => toggleSort("name")}
            >
              <span>Name</span>
              {sortBy === "name" ? (
                sortDesc ? <ArrowDown className="ml-1.5 h-3.5 w-3.5 shrink-0 text-foreground" /> : <ArrowUp className="ml-1.5 h-3.5 w-3.5 shrink-0 text-foreground" />
              ) : (
                <ArrowUpDown className="ml-1.5 h-3.5 w-3.5 shrink-0 opacity-40" />
              )}
            </div>
            <div 
              className="w-24 shrink-0 flex items-center justify-end pr-2 cursor-pointer hover:text-foreground transition-colors" 
              onClick={() => toggleSort("size")}
            >
              <span>Size</span>
              {sortBy === "size" ? (
                sortDesc ? <ArrowDown className="ml-1.5 h-3.5 w-3.5 shrink-0 text-foreground" /> : <ArrowUp className="ml-1.5 h-3.5 w-3.5 shrink-0 text-foreground" />
              ) : (
                <ArrowUpDown className="ml-1.5 h-3.5 w-3.5 shrink-0 opacity-40" />
              )}
            </div>
            <div className="w-24 shrink-0 hidden sm:flex items-center">
              Type
            </div>
            <div className="w-9 shrink-0 text-center" />
          </div>

          {loading ? (
            <div className="divide-y">
              {Array.from({ length: 12 }).map((_, i) => (
                <div key={i} className="flex items-center gap-3 px-4 py-2.5 border-b animate-pulse min-w-[500px]">
                  <div className="w-9 shrink-0 flex items-center justify-center">
                    <Skeleton className="h-4 w-4 rounded" />
                  </div>
                  <div className="flex-1 min-w-[160px]">
                    <Skeleton className="h-4 w-3/4 max-w-[280px]" />
                  </div>
                  <div className="w-24 shrink-0 pr-2 flex items-center justify-end">
                    <Skeleton className="h-4 w-14" />
                  </div>
                  <div className="w-24 shrink-0 hidden sm:flex items-center">
                    <Skeleton className="h-5 w-16 rounded-full" />
                  </div>
                  <div className="w-9 shrink-0 flex items-center justify-center">
                    <Skeleton className="h-7 w-7 rounded-md" />
                  </div>
                </div>
              ))}
            </div>
          ) : scanError ? (
            <div className="flex flex-col items-center justify-center h-full text-center p-8">
              <h3 className="text-lg font-medium">Couldn&apos;t scan cleanable items</h3>
              <p className="max-w-xl text-muted-foreground mt-1">{scanError}</p>
              <Button variant="outline" className="mt-4" onClick={() => void refreshCleaner()}>
                Try again
              </Button>
            </div>
          ) : filteredAndSortedItems.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-center p-8">
              <h3 className="text-lg font-medium">No items found</h3>
              <p className="text-muted-foreground mt-1">Your system is clean or the search returned no results.</p>
            </div>
          ) : (
            <div>
              <div className="divide-y pb-6">
                {visibleItems.map((item) => (
                  <div key={item.id} className="flex items-center gap-3 px-4 py-2.5 border-b hover:bg-muted/30 transition-colors min-w-[500px]">
                    <div className="w-9 shrink-0 flex items-center justify-center">
                      <Checkbox 
                        checked={selectedIds.has(item.id)}
                        onCheckedChange={() => toggleSelection(item.id)}
                        aria-label={`Select ${item.name}`}
                      />
                    </div>
                    <div className="flex-1 min-w-[160px] flex items-center min-w-0 overflow-hidden">
                      <span className="truncate font-medium text-sm select-text flex-1 min-w-0" title={item.name}>
                        {item.name}
                      </span>
                    </div>
                    <div className="w-24 shrink-0 font-medium text-xs sm:text-sm text-right pr-2 flex items-center justify-end">
                      {formatBytes(item.size_bytes)}
                    </div>
                    <div className="w-24 shrink-0 hidden sm:flex items-center overflow-hidden">
                      <Badge variant="outline" className={item.item_type === "Cache" ? "text-blue-500 bg-blue-500/10 border-blue-500/20 text-xs truncate max-w-full py-0 px-2 h-5" : "text-orange-500 bg-orange-500/10 border-orange-500/20 text-xs truncate max-w-full py-0 px-2 h-5"}>
                        <span className="truncate">{item.item_type}</span>
                      </Badge>
                    </div>
                    <div className="w-9 shrink-0 flex items-center justify-center">
                      <Button 
                        variant="ghost" 
                        size="icon" 
                        className="h-7 w-7 text-muted-foreground hover:text-foreground"
                        onClick={() => handleReveal(item.absolute_path)} 
                        title="Reveal in Finder"
                      >
                        <FolderSearch className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>

              {isStreaming && (
                <div className="flex items-center justify-center py-4 text-xs text-muted-foreground gap-2">
                  <span className="inline-block h-2 w-2 rounded-full bg-primary/60 animate-ping" />
                  Loading remaining items ({visibleCount} of {totalCount})...
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      <AlertDialog open={confirmOpen} onOpenChange={(open) => {
        if (!deleting) setConfirmOpen(open);
      }}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Clean selected items?</AlertDialogTitle>
            <AlertDialogDescription>
              This will move {selectedIds.size} selected cache or temporary {selectedIds.size === 1 ? "item" : "items"} ({formatBytes(totalSelectedSize)}) to your system Trash. You can restore them from there if needed.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deleting}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              disabled={deleting || selectedIds.size === 0}
              onClick={handleDeleteSelected}
            >
              {deleting ? "Cleaning..." : "Clean items"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
