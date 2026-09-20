use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Site {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
    pub default_entry_path: String,
    pub file_count: u32,
    pub total_size_bytes: u64,
    pub storage_key: Option<String>,
    pub deployed_by_user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeInfo {
    pub site: Site,
}

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("tenant not found: {0}")]
    TenantNotFound(Uuid),
    #[error("user not found: {0}")]
    UserNotFound(Uuid),
    #[error("site not found: {0}")]
    SiteNotFound(Uuid),
    #[error("user {user_id} does not belong to tenant {tenant_id}")]
    UserTenantMismatch { tenant_id: Uuid, user_id: Uuid },
    #[error("site {site_id} does not belong to tenant {tenant_id}")]
    SiteTenantMismatch { tenant_id: Uuid, site_id: Uuid },
}
