import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { DeletionResult } from "@/types/deletion";
import { FolderSearch, FolderPlus, File, ArrowUpDown, ArrowUp, ArrowDown, RefreshCw, X } from "lucide-react";
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

export function Files() {
  const items = useScanStore((state) => state.fileItems);
  const scanStatus = useScanStore((state) => state.filesStatus);
  const scanError = useScanStore((state) => state.filesError);
  const selectedFolders = useScanStore((state) => state.additionalFileFolders);
  const ensureFilesLoaded = useScanStore((state) => state.ensureFilesLoaded);
  const refreshFiles = useScanStore((state) => state.refreshFiles);
  const addAdditionalFileFolders = useScanStore((state) => state.addAdditionalFileFolders);
  const removeAdditionalFileFolder = useScanStore((state) => state.removeAdditionalFileFolder);
  const removeFilePaths = useScanStore((state) => state.removeFilePaths);
  const [search, setSearch] = useState("");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [deleting, setDeleting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [sortBy, setSortBy] = useState<"size" | "name" | "date">("size");
  const [sortDesc, setSortDesc] = useState(true);
  const [categoryFilter, setCategoryFilter] = useState<string>("all");

  useEffect(() => {
    // Fallback for this route being mounted outside the normal app shell.
    // In the normal flow AppLayout starts this scan before navigation.
    void ensureFilesLoaded();
  }, [ensureFilesLoaded]);

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

  const formatTimeAgo = (unixSecs: number) => {
    if (unixSecs === 0) return "Unknown";
    const now = Math.floor(Date.now() / 1000);
    const diff = now - unixSecs;
    
    if (diff < 60) return "Just now";
    if (diff < 3600) return `${Math.floor(diff / 60)} minutes ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} hours ago`;
    if (diff < 2592000) return `${Math.floor(diff / 86400)} days ago`;
    if (diff < 31536000) return `${Math.floor(diff / 2592000)} months ago`;
    return `${Math.floor(diff / 31536000)} years ago`;
  };

  const filteredAndSortedItems = useMemo(() => {
    const term = search.toLowerCase();
    let filtered = items.filter(item => 
      item.name.toLowerCase().includes(term) || 
      item.extension.toLowerCase().includes(term)
    );
    
    if (categoryFilter !== "all") {
      filtered = filtered.filter(item => item.category === categoryFilter);
    }

    return filtered.sort((a, b) => {
      let comparison = 0;
      if (sortBy === "size") {
        comparison = a.size_bytes - b.size_bytes;
      } else if (sortBy === "date") {
        comparison = a.last_modified - b.last_modified;
      } else {
        comparison = a.name.localeCompare(b.name);
      }
      return sortDesc ? -comparison : comparison;
    });
  }, [items, search, sortBy, sortDesc, categoryFilter]);

  const { visibleItems, visibleCount, totalCount, isStreaming, loadMore } =
    useProgressiveList(filteredAndSortedItems, {
      initialBatchSize: 30,
      streamBatchSize: 25,
      tickMs: 25,
      resetKey: `${search}-${categoryFilter}-${sortBy}-${sortDesc}`,
    });

  const categories = useMemo(() => {
    return ["all", ...Array.from(new Set(items.map((item) => item.category))).sort((a, b) => a.localeCompare(b))];
  }, [items]);

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
      // Re-use cleaner reveal command
      await invoke("reveal_in_finder", { path });
    } catch (err) {
      toast.error(`Failed to reveal file: ${err}`);
    }
  };

  const handleChooseFolders = async () => {
    try {
      const result = await open({
        title: "Choose folders to scan",
        directory: true,
        multiple: true,
        recursive: true,
        fileAccessMode: "scoped",
      });
      const paths = result === null ? [] : Array.isArray(result) ? result : [result];
      if (paths.length === 0) return;

      setSelectedIds(new Set());
      await addAdditionalFileFolders(paths);
    } catch (err) {
      toast.error(`Could not choose folders: ${err}`);
    }
  };

  const removeSelectedFolder = async (path: string) => {
    setSelectedIds(new Set());
    await removeAdditionalFileFolder(path);
  };

  const handleDeleteSelected = async () => {
    if (selectedIds.size === 0) return;
    
    const pathsToDelete = items
      .filter(i => selectedIds.has(i.id))
      .map(i => i.absolute_path);
      
    setDeleting(true);
    try {
      const result = await invoke<DeletionResult>("delete_files", { paths: pathsToDelete });
      if (result.deleted.length > 0) {
        toast.success(`Moved ${result.deleted.length} ${result.deleted.length === 1 ? "file" : "files"} to Trash.`);
      }
      if (result.failures.length > 0) {
        toast.warning(`${result.failures.length} ${result.failures.length === 1 ? "file could" : "files could"} not be moved: ${result.failures[0].reason}`);
      }
      setSelectedIds(new Set());
      setConfirmOpen(false);
      removeFilePaths(result.deleted);
    } catch (err) {
      toast.error(`Failed to delete files: ${err}`);
    } finally {
      setDeleting(false);
    }
  };

  const totalSelectedSize = useMemo(() => {
    return items
      .filter(i => selectedIds.has(i.id))
      .reduce((sum, item) => sum + item.size_bytes, 0);
  }, [items, selectedIds]);

  const toggleSort = (field: "size" | "name" | "date") => {
    if (sortBy === field) {
      setSortDesc(!sortDesc);
    } else {
      setSortBy(field);
      setSortDesc(true);
    }
  };

  return (
    <div className="flex flex-col gap-6 h-full min-h-0">
      {/* Standardized Reusable Page Header */}
      <PageHeader
        title="Large & Old Files"
        description="Scans visible folders in your home directory to reclaim disk space."
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
                {deleting ? "Deleting..." : `Delete Selected (${formatBytes(totalSelectedSize)})`}
              </Button>
            )}
            <Button
              variant="outline"
              size="sm"
              className="h-9 gap-1.5 text-xs font-medium shrink-0"
              onClick={handleChooseFolders}
              disabled={deleting}
              title="Add folders to scan"
            >
              <FolderPlus className="h-3.5 w-3.5" />
              Add folders
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="h-9 gap-1.5 text-xs font-medium shrink-0"
              onClick={() => void refreshFiles()}
              disabled={loading || deleting}
              title="Rescan files"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
              Refresh
            </Button>
          </>
        }
        filters={
          <div className="flex items-center gap-1 overflow-x-auto rounded-md border bg-muted/60 p-0.5 no-scrollbar max-w-full">
            {categories.map((category) => (
              <Button
                key={category}
                variant={categoryFilter === category ? "secondary" : "ghost"}
                size="sm"
                className="h-7 shrink-0 px-2.5 text-xs font-medium"
                onClick={() => setCategoryFilter(category)}
              >
                {category === "all" ? "All folders" : category}
              </Button>
            ))}
          </div>
        }
        sort={
          <div className="flex rounded-md border bg-muted/60 p-0.5 shrink-0">
            <Button
              variant={sortBy === "size" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-2 text-xs font-medium"
              onClick={() => toggleSort("size")}
              title="Sort by size"
            >
              Size {sortBy === "size" ? (sortDesc ? "↓" : "↑") : ""}
            </Button>
            <Button
              variant={sortBy === "date" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-2 text-xs font-medium"
              onClick={() => toggleSort("date")}
              title="Sort by date"
            >
              Date {sortBy === "date" ? (sortDesc ? "↓" : "↑") : ""}
            </Button>
            <Button
              variant={sortBy === "name" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 px-2 text-xs font-medium"
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
          placeholder: "Search files...",
        }}
        extra={
          selectedFolders.length > 0 ? (
            <div className="flex flex-wrap items-center gap-2 rounded-md border bg-muted/30 p-2 text-xs">
              <span className="text-muted-foreground shrink-0">Additional folders:</span>
              {selectedFolders.map((folder) => (
                <Badge key={folder} variant="secondary" className="max-w-full gap-1 pr-1">
                  <span className="truncate max-w-64" title={folder}>{folder}</span>
                  <button
                    type="button"
                    className="rounded-sm p-0.5 hover:bg-muted-foreground/20"
                    onClick={() => void removeSelectedFolder(folder)}
                    aria-label={`Stop scanning ${folder}`}
                  >
                    <X className="h-3 w-3" />
                  </button>
                </Badge>
              ))}
            </div>
          ) : null
        }
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
          {/* Sticky Table Header - inside same scroll container to guarantee exact alignment */}
          <div className="sticky top-0 z-20 flex items-center gap-3 px-4 py-2.5 border-b bg-muted/95 backdrop-blur-md font-medium text-xs sm:text-sm text-muted-foreground select-none min-w-[600px]">
            <div className="w-9 shrink-0 flex items-center justify-center">
              <Checkbox 
                checked={filteredAndSortedItems.length > 0 && selectedIds.size === filteredAndSortedItems.length}
                onCheckedChange={toggleAll}
                aria-label="Select all files"
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
              className="w-28 shrink-0 hidden md:flex items-center cursor-pointer hover:text-foreground transition-colors" 
              onClick={() => toggleSort("date")}
            >
              <span>Date</span>
              {sortBy === "date" ? (
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
            <div className="w-28 shrink-0 hidden lg:flex items-center">
              Category
            </div>
            <div className="w-9 shrink-0 text-center" />
          </div>

          {loading ? (
            <div className="divide-y">
              {Array.from({ length: 12 }).map((_, i) => (
                <div key={i} className="flex items-center gap-3 px-4 py-2.5 border-b animate-pulse min-w-[600px]">
                  <div className="w-9 shrink-0 flex items-center justify-center">
                    <Skeleton className="h-4 w-4 rounded" />
                  </div>
                  <div className="flex-1 min-w-[160px] flex items-center gap-2.5 min-w-0">
                    <Skeleton className="h-7 w-7 rounded-md shrink-0" />
                    <Skeleton className="h-4 w-3/4 max-w-[280px]" />
                  </div>
                  <div className="w-28 shrink-0 hidden md:flex items-center">
                    <Skeleton className="h-4 w-20" />
                  </div>
                  <div className="w-24 shrink-0 pr-2 flex items-center justify-end">
                    <Skeleton className="h-4 w-14" />
                  </div>
                  <div className="w-28 shrink-0 hidden lg:flex items-center">
                    <Skeleton className="h-5 w-20 rounded-full" />
                  </div>
                  <div className="w-9 shrink-0 flex items-center justify-center">
                    <Skeleton className="h-7 w-7 rounded-md" />
                  </div>
                </div>
              ))}
            </div>
          ) : scanError ? (
            <div className="flex flex-col items-center justify-center h-full text-center p-8">
              <h3 className="text-lg font-medium">Couldn&apos;t scan files</h3>
              <p className="max-w-xl text-muted-foreground mt-1">{scanError}</p>
              <Button variant="outline" className="mt-4" onClick={() => void refreshFiles()}>
                Try again
              </Button>
            </div>
          ) : filteredAndSortedItems.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-center p-8">
              <h3 className="text-lg font-medium">No large files found</h3>
              <p className="text-muted-foreground mt-1">Files of 1 MB or more in your visible home folders will appear here. Use Add folders to include another location.</p>
            </div>
          ) : (
            <div>
              <div className="divide-y pb-6">
                {visibleItems.map((item) => (
                  <div key={item.id} className="flex items-center gap-3 px-4 py-2.5 border-b hover:bg-muted/30 transition-colors min-w-[600px]">
                    <div className="w-9 shrink-0 flex items-center justify-center">
                      <Checkbox 
                        checked={selectedIds.has(item.id)}
                        onCheckedChange={() => toggleSelection(item.id)}
                        aria-label={`Select ${item.name}`}
                      />
                    </div>
                    <div className="flex-1 min-w-[160px] flex items-center gap-2.5 min-w-0 overflow-hidden">
                      <div className="shrink-0 p-1.5 bg-muted rounded-md text-muted-foreground">
                        <File className="h-4 w-4" />
                      </div>
                      <span className="truncate font-medium text-sm select-text flex-1 min-w-0" title={item.name}>
                        {item.name}
                      </span>
                    </div>
                    <div className="w-28 shrink-0 text-muted-foreground text-xs sm:text-sm truncate hidden md:flex items-center">
                      {formatTimeAgo(item.last_modified)}
                    </div>
                    <div className="w-24 shrink-0 font-medium text-xs sm:text-sm text-right pr-2 flex items-center justify-end">
                      {formatBytes(item.size_bytes)}
                    </div>
                    <div className="w-28 shrink-0 hidden lg:flex items-center overflow-hidden">
                      <Badge variant="outline" className="text-xs max-w-full truncate py-0 px-2 h-5" title={item.category}>
                        <span className="truncate">{item.category}</span>
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
                  Loading remaining files ({visibleCount} of {totalCount})...
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
            <AlertDialogTitle>Delete selected files?</AlertDialogTitle>
            <AlertDialogDescription>
              This will move {selectedIds.size} selected {selectedIds.size === 1 ? "file" : "files"} ({formatBytes(totalSelectedSize)}) to your system Trash. You can restore them from there if needed.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deleting}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              disabled={deleting || selectedIds.size === 0}
              onClick={handleDeleteSelected}
            >
              {deleting ? "Deleting..." : "Delete files"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
