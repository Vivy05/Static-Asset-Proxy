use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use domain::{DomainError, RuntimeInfo, Site, Tenant, User};
use persistence::{RepositoryError, StaticAssetRepository};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterTenantInput {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUserInput {
    pub tenant_id: Uuid,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSiteInput {
    pub tenant_id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
    pub default_entry_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploySiteInput {
    pub tenant_id: Uuid,
    pub deployed_by_user_id: Uuid,
    pub site_id: Uuid,
    pub storage_key: String,
    pub file_count: u32,
    pub total_size_bytes: u64,
}

#[derive(Debug, Default)]
struct InMemoryState {
    tenants: HashMap<Uuid, Tenant>,
    users: HashMap<Uuid, User>,
    sites: HashMap<Uuid, Site>,
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryRepository {
    inner: Arc<RwLock<InMemoryState>>,
}

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Domain(#[from] DomainError),
}

pub struct StaticAssetService<R>
where
    R: StaticAssetRepository,
{
    repository: Arc<R>,
}

impl<R> StaticAssetService<R>
where
    R: StaticAssetRepository,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn register_tenant(
        &self,
        input: RegisterTenantInput,
    ) -> Result<Tenant, ApplicationError> {
        if input.name.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "tenant name cannot be empty",
            ));
        }
        if input.slug.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "tenant slug cannot be empty",
            ));
        }

        self.repository
            .insert_tenant(Tenant {
                id: Uuid::new_v4(),
                name: input.name,
                slug: input.slug,
            })
            .await
            .map_err(ApplicationError::from)
    }

    pub async fn register_user(&self, input: RegisterUserInput) -> Result<User, ApplicationError> {
        if input.email.trim().is_empty() {
            return Err(ApplicationError::InvalidInput("user email cannot be empty"));
        }
        if input.display_name.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "user display name cannot be empty",
            ));
        }
        if self.repository.get_tenant(input.tenant_id).await?.is_none() {
            return Err(DomainError::TenantNotFound(input.tenant_id).into());
        }

        self.repository
            .insert_user(User {
                id: Uuid::new_v4(),
                tenant_id: input.tenant_id,
                email: input.email,
                display_name: input.display_name,
            })
            .await
            .map_err(ApplicationError::from)
    }

    pub async fn register_site(&self, input: RegisterSiteInput) -> Result<Site, ApplicationError> {
        if input.name.trim().is_empty() {
            return Err(ApplicationError::InvalidInput("site name cannot be empty"));
        }
        let tenant = self
            .repository
            .get_tenant(input.tenant_id)
            .await?
            .ok_or(DomainError::TenantNotFound(input.tenant_id))?;
        let owner = self
            .repository
            .get_user(input.owner_user_id)
            .await?
            .ok_or(DomainError::UserNotFound(input.owner_user_id))?;
        if owner.tenant_id != tenant.id {
            return Err(DomainError::UserTenantMismatch {
                tenant_id: tenant.id,
                user_id: owner.id,
            }
            .into());
        }

        self.repository
            .insert_site(Site {
                id: Uuid::new_v4(),
                tenant_id: input.tenant_id,
                owner_user_id: input.owner_user_id,
                name: input.name,
                default_entry_path: input.default_entry_path.unwrap_or_else(|| "/".to_string()),
                file_count: 0,
                total_size_bytes: 0,
                storage_key: None,
                deployed_by_user_id: None,
            })
            .await
            .map_err(ApplicationError::from)
    }

    pub async fn deploy_site(&self, input: DeploySiteInput) -> Result<Site, ApplicationError> {
        if input.storage_key.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "storage key cannot be empty",
            ));
        }
        if input.file_count == 0 {
            return Err(ApplicationError::InvalidInput(
                "deployed file count must be greater than zero",
            ));
        }
        if input.total_size_bytes == 0 {
            return Err(ApplicationError::InvalidInput(
                "deployed total size must be greater than zero",
            ));
        }

        let user = self
            .repository
            .get_user(input.deployed_by_user_id)
            .await?
            .ok_or(DomainError::UserNotFound(input.deployed_by_user_id))?;
        if user.tenant_id != input.tenant_id {
            return Err(DomainError::UserTenantMismatch {
                tenant_id: input.tenant_id,
                user_id: user.id,
            }
            .into());
        }

        let mut site = self
            .repository
            .get_site(input.site_id)
            .await?
            .ok_or(DomainError::SiteNotFound(input.site_id))?;
        if site.tenant_id != input.tenant_id {
            return Err(DomainError::SiteTenantMismatch {
                tenant_id: input.tenant_id,
                site_id: input.site_id,
            }
            .into());
        }

        site.storage_key = Some(input.storage_key);
        site.file_count = input.file_count;
        site.total_size_bytes = input.total_size_bytes;
        site.deployed_by_user_id = Some(input.deployed_by_user_id);

        self.repository
            .update_site(site)
            .await
            .map_err(ApplicationError::from)
    }

    pub async fn get_runtime_info(&self, site_id: Uuid) -> Result<RuntimeInfo, ApplicationError> {
        let site = self
            .repository
            .get_site(site_id)
            .await?
            .ok_or(DomainError::SiteNotFound(site_id))?;

        Ok(RuntimeInfo { site })
    }
}

impl InMemoryRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl StaticAssetRepository for InMemoryRepository {
    async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>, RepositoryError> {
        let state = self
            .inner
            .read()
            .map_err(|_| RepositoryError::Unavailable("state lock poisoned"))?;
        Ok(state.tenants.get(&tenant_id).cloned())
    }

    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>, RepositoryError> {
        let state = self
            .inner
            .read()
            .map_err(|_| RepositoryError::Unavailable("state lock poisoned"))?;
        Ok(state.users.get(&user_id).cloned())
    }

    async fn insert_tenant(&self, tenant: Tenant) -> Result<Tenant, RepositoryError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| RepositoryError::Unavailable("state lock poisoned"))?;
        state.tenants.insert(tenant.id, tenant.clone());
        Ok(tenant)
    }

    async fn insert_user(&self, user: User) -> Result<User, RepositoryError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| RepositoryError::Unavailable("state lock poisoned"))?;
        state.users.insert(user.id, user.clone());
        Ok(user)
    }

    async fn insert_site(&self, site: Site) -> Result<Site, RepositoryError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| RepositoryError::Unavailable("state lock poisoned"))?;
        state.sites.insert(site.id, site.clone());
        Ok(site)
    }

    async fn get_site(&self, site_id: Uuid) -> Result<Option<Site>, RepositoryError> {
        let state = self
            .inner
            .read()
            .map_err(|_| RepositoryError::Unavailable("state lock poisoned"))?;
        Ok(state.sites.get(&site_id).cloned())
    }

    async fn update_site(&self, site: Site) -> Result<Site, RepositoryError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| RepositoryError::Unavailable("state lock poisoned"))?;
        state.sites.insert(site.id, site.clone());
        Ok(site)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        DeploySiteInput, InMemoryRepository, RegisterSiteInput, RegisterTenantInput,
        RegisterUserInput, StaticAssetService,
    };
    use domain::DomainError;

    #[tokio::test]
    async fn deploys_site_and_exposes_runtime_info() {
        let service = StaticAssetService::new(Arc::new(InMemoryRepository::new()));
        let tenant = service
            .register_tenant(RegisterTenantInput {
                name: "Acme".to_string(),
                slug: "acme".to_string(),
            })
            .await
            .expect("tenant registration should succeed");
        let user = service
            .register_user(RegisterUserInput {
                tenant_id: tenant.id,
                email: "ops@acme.test".to_string(),
                display_name: "Acme Ops".to_string(),
            })
            .await
            .expect("user registration should succeed");
        let site = service
            .register_site(RegisterSiteInput {
                tenant_id: tenant.id,
                owner_user_id: user.id,
                name: "docs".to_string(),
                default_entry_path: Some("/".to_string()),
            })
            .await
            .expect("site registration should succeed");

        let deployed = service
            .deploy_site(DeploySiteInput {
                tenant_id: tenant.id,
                deployed_by_user_id: user.id,
                site_id: site.id,
                storage_key: "site/docs/current".to_string(),
                file_count: 3,
                total_size_bytes: 2048,
            })
            .await
            .expect("site deploy should succeed");

        let runtime = service
            .get_runtime_info(site.id)
            .await
            .expect("runtime info should succeed");

        assert_eq!(deployed.storage_key.as_deref(), Some("site/docs/current"));
        assert_eq!(runtime.site.file_count, 3);
        assert_eq!(runtime.site.total_size_bytes, 2048);
    }

    #[tokio::test]
    async fn rejects_deploy_when_user_is_outside_tenant() {
        let service = StaticAssetService::new(Arc::new(InMemoryRepository::new()));
        let tenant_a = service
            .register_tenant(RegisterTenantInput {
                name: "Acme".to_string(),
                slug: "acme".to_string(),
            })
            .await
            .expect("tenant registration should succeed");
        let tenant_b = service
            .register_tenant(RegisterTenantInput {
                name: "Beta".to_string(),
                slug: "beta".to_string(),
            })
            .await
            .expect("tenant registration should succeed");
        let owner = service
            .register_user(RegisterUserInput {
                tenant_id: tenant_a.id,
                email: "owner@acme.test".to_string(),
                display_name: "Owner".to_string(),
            })
            .await
            .expect("user registration should succeed");
        let outsider = service
            .register_user(RegisterUserInput {
                tenant_id: tenant_b.id,
                email: "ops@beta.test".to_string(),
                display_name: "Beta Ops".to_string(),
            })
            .await
            .expect("user registration should succeed");
        let site = service
            .register_site(RegisterSiteInput {
                tenant_id: tenant_a.id,
                owner_user_id: owner.id,
                name: "docs".to_string(),
                default_entry_path: None,
            })
            .await
            .expect("site registration should succeed");

        let result = service
            .deploy_site(DeploySiteInput {
                tenant_id: tenant_a.id,
                deployed_by_user_id: outsider.id,
                site_id: site.id,
                storage_key: "site/docs/current".to_string(),
                file_count: 1,
                total_size_bytes: 128,
            })
            .await
            .expect_err("cross tenant deploy should fail");

        match result {
            super::ApplicationError::Domain(DomainError::UserTenantMismatch {
                tenant_id,
                user_id,
            }) => {
                assert_eq!(tenant_id, tenant_a.id);
                assert_eq!(user_id, outsider.id);
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
