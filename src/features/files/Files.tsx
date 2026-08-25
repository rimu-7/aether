import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { FileItem } from "@/types/file";
import { Search, FolderSearch, File, ArrowUpDown } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";

export function Files() {
  const [items, setItems] = useState<FileItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [deleting, setDeleting] = useState(false);
  const [sortBy, setSortBy] = useState<"size" | "name" | "date">("size");
  const [sortDesc, setSortDesc] = useState(true);
  const [categoryFilter, setCategoryFilter] = useState<string>("all");

  const fetchItems = () => {
    setLoading(true);
    invoke<FileItem[]>("scan_large_files")
      .then(setItems)
      .catch((err) => toast.error(`Failed to scan files: ${err}`))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    fetchItems();
  }, []);

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
      toast.error(`Failed to open Finder: ${err}`);
    }
  };

  const handleDeleteSelected = async () => {
    if (selectedIds.size === 0) return;
    
    const pathsToDelete = items
      .filter(i => selectedIds.has(i.id))
      .map(i => i.absolute_path);
      
    setDeleting(true);
    try {
      const deleted = await invoke<string[]>("delete_files", { paths: pathsToDelete });
      toast.success(`Successfully deleted ${deleted.length} files`);
      setSelectedIds(new Set());
      fetchItems();
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
      <div className="flex items-center justify-between shrink-0">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Large & Old Files</h1>
          <p className="text-muted-foreground">Find forgotten files taking up massive amounts of space.</p>
        </div>
        <div className="flex gap-2">
          <Button 
            variant="destructive" 
            disabled={selectedIds.size === 0 || deleting}
            onClick={handleDeleteSelected}
          >
            {deleting ? "Deleting..." : `Delete Selected (${formatBytes(totalSelectedSize)})`}
          </Button>
        </div>
      </div>

      <div className="flex items-center justify-between shrink-0">
        <div className="flex items-center gap-4">
          <div className="flex rounded-md border bg-muted p-1">
            <Button 
              variant={categoryFilter === "all" ? "secondary" : "ghost"} 
              size="sm" 
              className="h-7 px-3 text-xs" 
              onClick={() => setCategoryFilter("all")}
            >
              All
            </Button>
            <Button 
              variant={categoryFilter === "Downloads" ? "secondary" : "ghost"} 
              size="sm" 
              className="h-7 px-3 text-xs" 
              onClick={() => setCategoryFilter("Downloads")}
            >
              Downloads
            </Button>
            <Button 
              variant={categoryFilter === "Documents" ? "secondary" : "ghost"} 
              size="sm" 
              className="h-7 px-3 text-xs" 
              onClick={() => setCategoryFilter("Documents")}
            >
              Documents
            </Button>
            <Button 
              variant={categoryFilter === "Movies" ? "secondary" : "ghost"} 
              size="sm" 
              className="h-7 px-3 text-xs" 
              onClick={() => setCategoryFilter("Movies")}
            >
              Movies
            </Button>
          </div>
          <div className="relative w-64">
            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input 
              className="pl-9" 
              placeholder="Search files..." 
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
        </div>
      </div>

      <div className="flex-1 border rounded-md overflow-hidden flex flex-col min-h-0">
        <div className="grid grid-cols-12 gap-4 p-4 border-b bg-muted/50 font-medium text-sm text-muted-foreground shrink-0">
          <div className="col-span-1 flex items-center justify-center">
            <Checkbox 
              checked={filteredAndSortedItems.length > 0 && selectedIds.size === filteredAndSortedItems.length}
              onCheckedChange={toggleAll}
            />
          </div>
          <div className="col-span-5 flex items-center cursor-pointer hover:text-foreground transition-colors" onClick={() => toggleSort("name")}>
            Name <ArrowUpDown className="ml-2 h-3 w-3" />
          </div>
          <div className="col-span-2 flex items-center cursor-pointer hover:text-foreground transition-colors" onClick={() => toggleSort("date")}>
            Date <ArrowUpDown className="ml-2 h-3 w-3" />
          </div>
          <div className="col-span-2 flex items-center cursor-pointer hover:text-foreground transition-colors" onClick={() => toggleSort("size")}>
            Size <ArrowUpDown className="ml-2 h-3 w-3" />
          </div>
          <div className="col-span-1 flex items-center">
            Category
          </div>
          <div className="col-span-1 text-center">
            Action
          </div>
        </div>

        <div className="flex-1 overflow-y-auto">
          {loading ? (
            <div className="p-4 space-y-4">
              {Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="flex items-center gap-4">
                  <Skeleton className="h-4 w-4 rounded" />
                  <Skeleton className="h-8 w-8 rounded-md" />
                  <Skeleton className="h-5 flex-1" />
                  <Skeleton className="h-5 w-24" />
                  <Skeleton className="h-8 w-8 rounded-md" />
                </div>
              ))}
            </div>
          ) : filteredAndSortedItems.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-center p-8">
              <h3 className="text-lg font-medium">No files found</h3>
              <p className="text-muted-foreground mt-1">Your system is clean or the search returned no results.</p>
            </div>
          ) : (
            <div className="divide-y pb-6">
              {filteredAndSortedItems.map((item) => (
                <div key={item.id} className="grid grid-cols-12 gap-4 p-4 items-center hover:bg-muted/30 transition-colors">
                  <div className="col-span-1 flex items-center justify-center">
                    <Checkbox 
                      checked={selectedIds.has(item.id)}
                      onCheckedChange={() => toggleSelection(item.id)}
                    />
                  </div>
                  <div className="col-span-5 flex items-center gap-3 overflow-hidden">
                    <div className="shrink-0 p-2 bg-muted rounded-md text-muted-foreground">
                      <File className="h-4 w-4" />
                    </div>
                    <div className="truncate font-medium flex-1" title={item.name}>
                      {item.name}
                    </div>
                  </div>
                  <div className="col-span-2 text-muted-foreground text-sm">
                    {formatTimeAgo(item.last_modified)}
                  </div>
                  <div className="col-span-2 font-medium">
                    {formatBytes(item.size_bytes)}
                  </div>
                  <div className="col-span-1">
                    <Badge variant="outline" className="text-xs">
                      {item.category}
                    </Badge>
                  </div>
                  <div className="col-span-1 flex items-center justify-center">
                    <Button variant="ghost" size="icon" onClick={() => handleReveal(item.absolute_path)} title="Reveal in Finder">
                      <FolderSearch className="h-4 w-4 text-muted-foreground" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
