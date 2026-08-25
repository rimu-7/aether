use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub bundle_id: Option<String>,
    pub name: String,
    pub display_name: String,
    pub version: Option<String>,
    pub developer: Option<String>,
    pub bundle_path: String,
    pub executable_path: Option<String>,
    pub icon_path: Option<String>,
    pub is_system: bool,
    pub is_running: bool,
    pub size_bytes: u64,
}
