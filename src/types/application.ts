export interface Application {
  id: string;
  bundle_id?: string;
  name: string;
  display_name: string;
  version?: string;
  developer?: string;
  bundle_path: string;
  executable_path?: string;
  icon_path?: string;
  is_system: boolean;
  is_running: boolean;
  size_bytes: number;
}

export type ArtifactConfidence = "Unknown" | "Low" | "Medium" | "High" | "Exact";

export interface Artifact {
  path: string;
  category: string;
  confidence: ArtifactConfidence;
  size_bytes: number;
}
