use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub is_cask: bool,
}
