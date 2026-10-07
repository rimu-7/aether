export interface Package {
  id: string;
  name: string;
  description?: string;
  version: string;
  is_cask: boolean;
  manager: string;
  size_bytes: number;
}
