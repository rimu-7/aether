export interface FileItem {
  id: string;
  name: string;
  absolute_path: string;
  size_bytes: number;
  last_modified: number;
  extension: string;
  category: string;
}
