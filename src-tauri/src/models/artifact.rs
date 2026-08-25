use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArtifactConfidence {
    Unknown,
    Low,
    Medium,
    High,
    Exact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub category: String,
    pub confidence: ArtifactConfidence,
    pub size_bytes: u64,
}
