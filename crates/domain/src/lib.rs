use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Site {
    pub id: Uuid,
    pub name: String,
    pub default_entry_path: String,
    pub active_version_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactVersion {
    pub id: Uuid,
    pub site_id: Uuid,
    pub label: String,
    pub storage_key: String,
    pub entry_file: String,
    pub file_count: u32,
    pub total_size_bytes: u64,
    pub status: ArtifactStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactStatus {
    Uploaded,
    Active,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeInfo {
    pub site: Site,
    pub active_version: Option<ArtifactVersion>,
    pub versions: Vec<ArtifactVersion>,
}

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("site not found: {0}")]
    SiteNotFound(Uuid),
    #[error("artifact version not found: {0}")]
    ArtifactVersionNotFound(Uuid),
    #[error("artifact version {version_id} does not belong to site {site_id}")]
    VersionSiteMismatch { site_id: Uuid, version_id: Uuid },
    #[error("artifact label already exists for site {site_id}: {label}")]
    DuplicateArtifactLabel { site_id: Uuid, label: String },
}
