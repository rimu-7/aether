use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub id: String,
    pub name: String,
    pub absolute_path: String,
    pub size_bytes: u64,
    pub last_modified: u64, // Unix timestamp
    pub extension: String,
    pub category: String, // e.g. "Downloads", "Documents"
}
