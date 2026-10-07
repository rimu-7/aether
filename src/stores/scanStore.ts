import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

import type { Application } from "@/types/application";
import type { CleanableItem } from "@/types/cleaner";
import type { FileItem } from "@/types/file";
import type { Package } from "@/types/package";

export type ScanStatus = "idle" | "loading" | "ready" | "error";

export interface ScanStore {
  fileItems: FileItem[];
  cleanableItems: CleanableItem[];
  applications: Application[];
  packages: Package[];

  filesStatus: ScanStatus;
  cleanerStatus: ScanStatus;
  applicationsStatus: ScanStatus;
  packagesStatus: ScanStatus;

  filesError: string | null;
  cleanerError: string | null;
  applicationsError: string | null;
  packagesError: string | null;

  additionalFileFolders: string[];

  /** Starts the default file scan once. Reuses its result on later calls. */
  ensureFilesLoaded: () => Promise<void>;
  /** Starts the cleaner scan once. Reuses its result on later calls. */
  ensureCleanerLoaded: () => Promise<void>;
  /** Starts the application scan once. Reuses its result on later calls. */
  ensureApplicationsLoaded: () => Promise<void>;
  /** Starts the package scan once. Reuses its result on later calls. */
  ensurePackagesLoaded: () => Promise<void>;
  /** Queues startup scans in the background so tabs open instantly. */
  ensureScansLoaded: () => Promise<void>;

  /** Explicitly scans files again. */
  refreshFiles: () => Promise<void>;
  /** Explicitly scans cleanable items again. */
  refreshCleaner: () => Promise<void>;
  /** Explicitly scans applications again. */
  refreshApplications: () => Promise<void>;
  /** Explicitly scans packages again. */
  refreshPackages: () => Promise<void>;

  /** Replaces the folders included in the file scan and refreshes that scan. */
  updateAdditionalFileFolders: (folders: string[]) => Promise<void>;
  /** Adds folders included in the file scan and refreshes that scan. */
  addAdditionalFileFolders: (folders: string[]) => Promise<void>;
  /** Stops including a folder in the file scan and refreshes that scan. */
  removeAdditionalFileFolder: (folder: string) => Promise<void>;

  /** Removes successfully deleted files from the cached list without rescanning. */
  removeFilePaths: (paths: string[]) => void;
  /** Removes successfully deleted cleanable items from the cached list without rescanning. */
  removeCleanablePaths: (paths: string[]) => void;
  /** Removes an uninstalled application from cache. */
  removeApplication: (id: string) => void;
  /** Removes an uninstalled package from cache. */
  removePackage: (id: string) => void;
}

let filesScanPromise: Promise<void> | null = null;
let filesScanFoldersKey: string | null = null;
let cleanerScanPromise: Promise<void> | null = null;
let applicationsScanPromise: Promise<void> | null = null;
let packagesScanPromise: Promise<void> | null = null;

const deletedFilePaths = new Set<string>();
const deletedCleanablePaths = new Set<string>();

const uniqueFolders = (folders: string[]) =>
  Array.from(new Set(folders.filter((folder) => folder.length > 0)));

const foldersKey = (folders: string[]) => folders.join("\u0000");

const sameFolders = (left: string[], right: string[]) =>
  left.length === right.length && left.every((folder, index) => folder === right[index]);

const errorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

export const useScanStore = create<ScanStore>((set, get) => {
  const scanFiles = (force: boolean): Promise<void> => {
    const folders = get().additionalFileFolders;
    const currentFoldersKey = foldersKey(folders);

    if (filesScanPromise) {
      if (filesScanFoldersKey !== currentFoldersKey) {
        return filesScanPromise.then(() => scanFiles(true));
      }
      return filesScanPromise;
    }

    if (!force && get().filesStatus === "ready") {
      return Promise.resolve();
    }

    if (force) {
      deletedFilePaths.clear();
    }

    set({ filesStatus: "loading", filesError: null });
    filesScanFoldersKey = currentFoldersKey;
    filesScanPromise = invoke<FileItem[]>("scan_large_files", {
      additionalPaths: folders,
    })
      .then((fileItems) => {
        set({
          fileItems: fileItems.filter(
            (item) => !deletedFilePaths.has(item.absolute_path),
          ),
          filesStatus: "ready",
          filesError: null,
        });
      })
      .catch((error: unknown) => {
        set({ filesStatus: "error", filesError: errorMessage(error) });
      })
      .finally(() => {
        filesScanPromise = null;
        filesScanFoldersKey = null;
      });

    return filesScanPromise;
  };

  const scanCleaner = (force: boolean): Promise<void> => {
    if (cleanerScanPromise) {
      return cleanerScanPromise;
    }

    if (!force && get().cleanerStatus === "ready") {
      return Promise.resolve();
    }

    if (force) {
      deletedCleanablePaths.clear();
    }

    set({ cleanerStatus: "loading", cleanerError: null });
    cleanerScanPromise = invoke<CleanableItem[]>("scan_cleanable_items")
      .then((cleanableItems) => {
        set({
          cleanableItems: cleanableItems.filter(
            (item) => !deletedCleanablePaths.has(item.absolute_path),
          ),
          cleanerStatus: "ready",
          cleanerError: null,
        });
      })
      .catch((error: unknown) => {
        set({ cleanerStatus: "error", cleanerError: errorMessage(error) });
      })
      .finally(() => {
        cleanerScanPromise = null;
      });

    return cleanerScanPromise;
  };

  const scanApplications = (force: boolean): Promise<void> => {
    if (applicationsScanPromise) {
      return applicationsScanPromise;
    }

    if (!force && get().applicationsStatus === "ready") {
      return Promise.resolve();
    }

    set({ applicationsStatus: "loading", applicationsError: null });
    applicationsScanPromise = invoke<Application[]>("scan_applications")
      .then((data) => {
        set({
          applications: data,
          applicationsStatus: "ready",
          applicationsError: null,
        });
      })
      .catch((error: unknown) => {
        set({ applicationsStatus: "error", applicationsError: errorMessage(error) });
      })
      .finally(() => {
        applicationsScanPromise = null;
      });

    return applicationsScanPromise;
  };

  const scanPackages = (force: boolean): Promise<void> => {
    if (packagesScanPromise) {
      return packagesScanPromise;
    }

    if (!force && get().packagesStatus === "ready") {
      return Promise.resolve();
    }

    set({ packagesStatus: "loading", packagesError: null });
    packagesScanPromise = invoke<Package[]>("get_installed_packages")
      .then((data) => {
        set({
          packages: data,
          packagesStatus: "ready",
          packagesError: null,
        });
      })
      .catch((error: unknown) => {
        set({ packagesStatus: "error", packagesError: errorMessage(error) });
      })
      .finally(() => {
        packagesScanPromise = null;
      });

    return packagesScanPromise;
  };

  return {
    fileItems: [],
    cleanableItems: [],
    applications: [],
    packages: [],

    filesStatus: "idle",
    cleanerStatus: "idle",
    applicationsStatus: "idle",
    packagesStatus: "idle",

    filesError: null,
    cleanerError: null,
    applicationsError: null,
    packagesError: null,

    additionalFileFolders: [],

    ensureFilesLoaded: () => scanFiles(false),
    ensureCleanerLoaded: () => scanCleaner(false),
    ensureApplicationsLoaded: () => scanApplications(false),
    ensurePackagesLoaded: () => scanPackages(false),

    ensureScansLoaded: async () => {
      // Warm up scans in the background on startup
      void scanApplications(false);
      void scanPackages(false);
      void scanCleaner(false);
      void scanFiles(false);
    },

    refreshFiles: () => scanFiles(true),
    refreshCleaner: () => scanCleaner(true),
    refreshApplications: () => scanApplications(true),
    refreshPackages: () => scanPackages(true),

    updateAdditionalFileFolders: (folders) => {
      const nextFolders = uniqueFolders(folders);
      if (sameFolders(nextFolders, get().additionalFileFolders)) {
        return Promise.resolve();
      }

      set({ additionalFileFolders: nextFolders });
      return scanFiles(true);
    },
    addAdditionalFileFolders: (folders) => {
      const nextFolders = uniqueFolders([
        ...get().additionalFileFolders,
        ...folders,
      ]);
      return get().updateAdditionalFileFolders(nextFolders);
    },
    removeAdditionalFileFolder: (folder) =>
      get().updateAdditionalFileFolders(
        get().additionalFileFolders.filter((currentFolder) => currentFolder !== folder),
      ),

    removeFilePaths: (paths) => {
      const deletedPaths = new Set(paths);
      if (deletedPaths.size === 0) return;

      paths.forEach((path) => deletedFilePaths.add(path));
      set((state) => ({
        fileItems: state.fileItems.filter(
          (item) => !deletedPaths.has(item.absolute_path),
        ),
      }));
    },
    removeCleanablePaths: (paths) => {
      const deletedPaths = new Set(paths);
      if (deletedPaths.size === 0) return;

      paths.forEach((path) => deletedCleanablePaths.add(path));
      set((state) => ({
        cleanableItems: state.cleanableItems.filter(
          (item) => !deletedPaths.has(item.absolute_path),
        ),
      }));
    },
    removeApplication: (id: string) => {
      set((state) => ({
        applications: state.applications.filter((app) => app.id !== id),
      }));
    },
    removePackage: (id: string) => {
      set((state) => ({
        packages: state.packages.filter((pkg) => pkg.id !== id),
      }));
    },
  };
});
