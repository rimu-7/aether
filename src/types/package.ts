export interface Package {
  id: string;
  name: string;
  description?: string;
  version: string;
  is_cask: boolean;
}
