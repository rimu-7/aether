import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Application } from "@/types/application";
import { ApplicationCard } from "./ApplicationCard";
import { UninstallDialog } from "./UninstallDialog";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";

export function Applications() {
  const [applications, setApplications] = useState<Application[]>([]);
  const [filteredApps, setFilteredApps] = useState<Application[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  
  const [selectedApp, setSelectedApp] = useState<Application | null>(null);

  const fetchApplications = () => {
    setLoading(true);
    invoke<Application[]>("scan_applications")
      .then((data) => {
        // Sort alphabetically
        data.sort((a, b) => a.display_name.localeCompare(b.display_name));
        setApplications(data);
        setFilteredApps(data);
      })
      .catch((err) => {
        toast.error(`Failed to scan applications: ${err}`);
      })
      .finally(() => {
        setLoading(false);
      });
  };

  useEffect(() => {
    fetchApplications();
  }, []);

  useEffect(() => {
    const term = search.toLowerCase();
    setFilteredApps(
      applications.filter(app => 
        app.display_name.toLowerCase().includes(term) || 
        app.bundle_id?.toLowerCase().includes(term)
      )
    );
  }, [search, applications]);

  return (
    <div className="flex flex-col gap-6 h-full min-h-0">
      <div className="flex items-center justify-between shrink-0">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Applications</h1>
          <p className="text-muted-foreground">Manage and uninstall applications installed on your system.</p>
        </div>
        <div className="w-64">
          <Input 
            placeholder="Search applications..." 
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto min-h-0 pr-2">
        {loading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {Array.from({ length: 12 }).map((_, i) => (
              <div key={i} className="flex flex-col p-4 border rounded-xl space-y-3">
                <div className="flex items-center gap-3">
                  <Skeleton className="w-12 h-12 rounded-md" />
                  <div className="space-y-2">
                    <Skeleton className="h-4 w-[150px]" />
                    <Skeleton className="h-3 w-[100px]" />
                  </div>
                </div>
                <div className="flex items-center justify-between mt-4">
                  <Skeleton className="h-5 w-[60px] rounded-full" />
                  <Skeleton className="h-4 w-[60px]" />
                </div>
              </div>
            ))}
          </div>
        ) : filteredApps.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <h3 className="text-lg font-medium">No applications found</h3>
            <p className="text-muted-foreground mt-1">Try adjusting your search query.</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 pb-12">
            {filteredApps.map((app) => (
              <ApplicationCard 
                key={app.id} 
                app={app} 
                onDeleteClick={setSelectedApp} 
              />
            ))}
          </div>
        )}
      </div>

      <UninstallDialog 
        app={selectedApp} 
        onOpenChange={(open) => !open && setSelectedApp(null)}
        onUninstallComplete={fetchApplications}
      />
    </div>
  );
}
