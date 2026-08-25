import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { CleanableItem } from "@/types/cleaner";
import { Search, FolderSearch, Trash2, ArrowUpDown } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";

export function Cleaner() {
  const [items, setItems] = useState<CleanableItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [deleting, setDeleting] = useState(false);
  const [sortBy, setSortBy] = useState<"size" | "name">("size");
  const [sortDesc, setSortDesc] = useState(true);

  const fetchItems = () => {
    setLoading(true);
    invoke<CleanableItem[]>("scan_cleanable_items")
      .then(setItems)
      .catch((err) => toast.error(`Failed to scan items: ${err}`))
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
      const deleted = await invoke<string[]>("delete_cleanable_items", { paths: pathsToDelete });
      toast.success(`Successfully cleaned ${deleted.length} items`);
      setSelectedIds(new Set());
      fetchItems();
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
      <div className="flex items-center justify-between shrink-0">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Deep Cleaner</h1>
          <p className="text-muted-foreground">Reclaim disk space by safely removing caches and logs.</p>
        </div>
        <div className="flex gap-2">
          <Button 
            variant="destructive" 
            disabled={selectedIds.size === 0 || deleting}
            onClick={handleDeleteSelected}
          >
            {deleting ? "Cleaning..." : `Clean Selected (${formatBytes(totalSelectedSize)})`}
          </Button>
        </div>
      </div>

      <div className="flex items-center justify-between shrink-0">
        <div className="flex items-center gap-4">
          <div className="relative w-64">
            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input 
              className="pl-9" 
              placeholder="Search caches..." 
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
          <div className="col-span-6 flex items-center cursor-pointer hover:text-foreground transition-colors" onClick={() => toggleSort("name")}>
            Name <ArrowUpDown className="ml-2 h-3 w-3" />
          </div>
          <div className="col-span-2 flex items-center cursor-pointer hover:text-foreground transition-colors" onClick={() => toggleSort("size")}>
            Size <ArrowUpDown className="ml-2 h-3 w-3" />
          </div>
          <div className="col-span-2 flex items-center">
            Type
          </div>
          <div className="col-span-1 text-center">
            Actions
          </div>
        </div>

        <div className="flex-1 overflow-y-auto">
          {loading ? (
            <div className="p-4 space-y-4">
              {Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="flex items-center gap-4">
                  <Skeleton className="h-4 w-4 rounded" />
                  <Skeleton className="h-5 flex-1" />
                  <Skeleton className="h-5 w-24" />
                  <Skeleton className="h-8 w-8 rounded-md" />
                </div>
              ))}
            </div>
          ) : filteredAndSortedItems.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-center p-8">
              <h3 className="text-lg font-medium">No items found</h3>
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
                  <div className="col-span-6 truncate font-medium" title={item.name}>
                    {item.name}
                  </div>
                  <div className="col-span-2 text-muted-foreground text-sm">
                    {formatBytes(item.size_bytes)}
                  </div>
                  <div className="col-span-2">
                    <Badge variant="outline" className={item.item_type === "Cache" ? "text-blue-500 bg-blue-500/10 border-blue-500/20" : "text-orange-500 bg-orange-500/10 border-orange-500/20"}>
                      {item.item_type}
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
