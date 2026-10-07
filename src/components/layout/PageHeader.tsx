import React from "react";
import { Search, X } from "lucide-react";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

export interface PageHeaderSearchProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export interface PageHeaderProps {
  title: string;
  description: string;
  badge?: React.ReactNode;
  actions?: React.ReactNode;
  filters?: React.ReactNode;
  sort?: React.ReactNode;
  search?: PageHeaderSearchProps;
  extra?: React.ReactNode;
  className?: string;
}

export function PageHeader({
  title,
  description,
  badge,
  actions,
  filters,
  sort,
  search,
  extra,
  className,
}: PageHeaderProps) {
  return (
    <div
      className={cn(
        "flex flex-col gap-4 shrink-0 select-none pb-2 transition-all duration-150",
        className,
      )}
    >
      {/* Top Row: Title & Subtitle on Left, Primary Actions on Right */}
      <div className="flex flex-wrap sm:flex-nowrap items-center justify-between gap-3 min-h-[52px]">
        <div className="flex flex-col justify-center min-w-0 flex-1">
          <div className="flex items-center gap-2.5 min-w-0">
            <h1 className="text-2xl sm:text-3xl font-bold tracking-tight text-foreground truncate">
              {title}
            </h1>
            {badge && <div className="shrink-0">{badge}</div>}
          </div>
          <p
            className="text-xs sm:text-sm text-muted-foreground truncate mt-0.5"
            title={description}
          >
            {description}
          </p>
        </div>

        <div className="flex items-center gap-2 shrink-0 flex-wrap sm:flex-nowrap">
          {actions}
        </div>
      </div>

      {/* Bottom Row: Filters/Tabs on Left, Sort + Search on Right */}
      {(filters || sort || search) && (
        <div className="flex flex-wrap sm:flex-nowrap items-center justify-between gap-2.5 min-h-[36px]">
          <div className="flex items-center gap-1.5 min-w-0 flex-1 overflow-x-auto no-scrollbar">
            {filters}
          </div>

          <div className="flex items-center gap-2 shrink-0 ml-auto">
            {sort}

            {search && (
              <div className="relative w-36 sm:w-44 md:w-52 shrink-0">
                <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground pointer-events-none" />
                <Input
                  className="h-9 pl-9 pr-8 text-sm bg-background/80"
                  placeholder={search.placeholder || "Search..."}
                  value={search.value}
                  onChange={(e) => search.onChange(e.target.value)}
                />
                {search.value && (
                  <button
                    type="button"
                    onClick={() => search.onChange("")}
                    className="absolute right-2.5 top-2.5 text-muted-foreground hover:text-foreground p-0.5 rounded-sm"
                    aria-label="Clear search"
                  >
                    <X className="h-4 w-4" />
                  </button>
                )}
              </div>
            )}
          </div>
        </div>
      )}

      {extra && <div className="pt-1">{extra}</div>}
    </div>
  );
}
