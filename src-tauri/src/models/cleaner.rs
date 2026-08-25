use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanableItem {
    pub id: String,
    pub name: String,
    pub absolute_path: String,
    pub size_bytes: u64,
    pub item_type: String, // "Cache", "Log", "Temp"
}
