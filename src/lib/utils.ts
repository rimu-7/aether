import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export type Platform = "macos" | "windows" | "linux" | "unknown";

export function detectPlatform(): Platform {
  if (typeof navigator === "undefined" || typeof navigator.userAgent === "undefined") {
    return "unknown";
  }
  const ua = navigator.userAgent;
  if (ua.includes("Mac")) return "macos";
  if (ua.includes("Win")) return "windows";
  if (ua.includes("Linux")) return "linux";
  return "unknown";
}

export function isMacOS(): boolean {
  return detectPlatform() === "macos";
}

export function isWindows(): boolean {
  return detectPlatform() === "windows";
}

export function isLinux(): boolean {
  return detectPlatform() === "linux";
}
