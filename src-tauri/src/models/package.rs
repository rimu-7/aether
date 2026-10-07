use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub is_cask: bool,
    /// The package source (`homebrew`, `apt`, `flatpak`, `windows-registry`,
    /// etc.). IDs are only meaningful within this source.
    pub manager: String,
    /// Installed size when the platform reports it. A zero value means the
    /// package manager does not expose a reliable size for this entry.
    pub size_bytes: u64,
}
