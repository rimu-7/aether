import { Outlet, NavLink } from "react-router-dom";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { load } from "@tauri-apps/plugin-store";

import {
  LayoutDashboard,
  AppWindow,
  Eraser,
  Package,
  File,
  Moon,
  Sun,
  Monitor,
  Activity,
  AlignHorizontalSpaceAround,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { useTheme } from "../theme-provider";

const navItems = [
  {
    name: "Dashboard",
    href: "/",
    icon: LayoutDashboard,
  },
  {
    name: "Applications",
    href: "/applications",
    icon: AppWindow,
  },
  {
    name: "Packages",
    href: "/packages",
    icon: Package,
  },
  {
    name: "Cleaner",
    href: "/cleaner",
    icon: Eraser,
  },
  {
    name: "Files",
    href: "/activity",
    icon: File,
  },
];

type MenubarSettings = {
  enabled: boolean;
};

export function AppLayout() {
  const { theme, setTheme } = useTheme();

  const [menubarSpeed, setMenubarSpeed] = useState(false);

  const [isLoadingMenubar, setIsLoadingMenubar] = useState(true);

  const [menubarError, setMenubarError] = useState<string | null>(null);

  // ============================================================
  // LOAD SETTINGS
  // ============================================================

  useEffect(() => {
    let mounted = true;

    const loadSettings = async () => {
      try {
        setIsLoadingMenubar(true);
        setMenubarError(null);

        const store = await load("settings.json");

        const saved = await store.get<MenubarSettings>("menubar_settings");

        const enabled = saved?.enabled ?? false;

        if (!mounted) {
          return;
        }

        setMenubarSpeed(enabled);

        // Synchronize frontend state with Rust.
        await invoke("update_menubar_settings", {
          enabled,
        });

        console.log("[Menubar] Settings restored:", {
          enabled,
        });
      } catch (error) {
        console.error("[Menubar] Failed to load settings:", error);

        if (mounted) {
          setMenubarError("Unable to initialize Speed Meter");
        }
      } finally {
        if (mounted) {
          setIsLoadingMenubar(false);
        }
      }
    };

    loadSettings();

    return () => {
      mounted = false;
    };
  }, []);

  // ============================================================
  // UPDATE SETTINGS
  // ============================================================

  const updateMenubarSettings = async (enabled: boolean) => {
    const previousValue = menubarSpeed;

    try {
      setMenubarError(null);

      // Optimistic UI update.
      setMenubarSpeed(enabled);

      const store = await load("settings.json");

      await store.set("menubar_settings", {
        enabled,
      });

      await store.save();

      // Tell Rust immediately.
      await invoke("update_menubar_settings", {
        enabled,
      });

      console.log("[Menubar] Updated:", {
        enabled,
      });
    } catch (error) {
      console.error("[Menubar] Failed to update:", error);

      // Roll UI back if Rust/store failed.
      setMenubarSpeed(previousValue);

      setMenubarError(String(error));
    }
  };

  // ============================================================
  // RENDER
  // ============================================================

  return (
    <div className="fixed inset-0 flex overflow-hidden rounded-xl bg-background text-foreground">
      {/* ========================================================
          SIDEBAR
      ======================================================== */}

      <aside className="flex h-full w-64 shrink-0 flex-col border-r bg-muted/30">
        {/* ======================================================
            BRAND
        ====================================================== */}

        {/* ======================================================
            NAVIGATION
        ====================================================== */}

        <nav className="flex flex-1 flex-col gap-1 overflow-y-auto px-3 py-4">
          {navItems.map((item) => {
            const Icon = item.icon;

            return (
              <NavLink
                key={item.name}
                to={item.href}
                className={({ isActive }) =>
                  cn(
                    "flex shrink-0 items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                    isActive
                      ? "bg-primary text-primary-foreground"
                      : "text-muted-foreground hover:bg-muted hover:text-foreground",
                  )
                }
              >
                <Icon className="h-4 w-4" />

                {item.name}
              </NavLink>
            );
          })}
        </nav>

        {/* ======================================================
            SETTINGS
        ====================================================== */}

        <div className="flex flex-col gap-4 border-t p-4">
          {/* ====================================================
              THEME
          ==================================================== */}

          <div className="flex items-center justify-between">
            <span className="text-sm font-medium text-muted-foreground">
              Theme
            </span>

            <div className="flex items-center rounded-md bg-muted p-1">
              <button
                type="button"
                aria-label="Light theme"
                onClick={() => setTheme("light")}
                className={cn(
                  "rounded-sm p-1.5 transition-all",
                  theme === "light"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                <Sun size={14} />
              </button>

              <button
                type="button"
                aria-label="System theme"
                onClick={() => setTheme("system")}
                className={cn(
                  "rounded-sm p-1.5 transition-all",
                  theme === "system"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                <Monitor size={14} />
              </button>

              <button
                type="button"
                aria-label="Dark theme"
                onClick={() => setTheme("dark")}
                className={cn(
                  "rounded-sm p-1.5 transition-all",
                  theme === "dark"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                <Moon size={14} />
              </button>
            </div>
          </div>

          {/* ====================================================
              SPEED METER
          ==================================================== */}

          <div className="flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <span className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                <Activity size={14} />
                Speed Meter
              </span>

              <button
                type="button"
                role="switch"
                aria-checked={menubarSpeed}
                aria-label="Toggle Speed Meter"
                disabled={isLoadingMenubar}
                onClick={() => updateMenubarSettings(!menubarSpeed)}
                className={cn(
                  "relative h-4 w-8 rounded-full transition-colors",
                  "disabled:cursor-not-allowed disabled:opacity-50",
                  menubarSpeed ? "bg-primary" : "bg-muted-foreground/30",
                )}
              >
                <span
                  className={cn(
                    "absolute top-0.5 h-3 w-3 rounded-full bg-white shadow-sm transition-all",
                    menubarSpeed ? "right-0.5" : "left-0.5",
                  )}
                />
              </button>
            </div>

            {/* ==================================================
                DESCRIPTION
            ================================================== */}

            <div className="flex items-center gap-2 pl-6">
              <AlignHorizontalSpaceAround
                size={12}
                className="text-muted-foreground"
              />

              <span className="text-xs text-muted-foreground">
                {menubarSpeed
                  ? "Showing network speed in menu bar"
                  : "Hidden from menu bar"}
              </span>
            </div>

            {/* ==================================================
                ERROR
            ================================================== */}

            {menubarError && (
              <div className="pl-6 text-xs text-destructive">
                {menubarError}
              </div>
            )}
          </div>
        </div>
      </aside>

      {/* ========================================================
          MAIN CONTENT
      ======================================================== */}

      <main className="relative flex h-full min-w-0 flex-1 flex-col bg-background/50">
        <div className="relative z-10 flex min-h-0 flex-1 flex-col overflow-hidden p-8 pt-12">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
