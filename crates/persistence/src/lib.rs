use async_trait::async_trait;
use domain::{Site, Tenant, User};
use sqlx::{PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("entity not found")]
    NotFound,
    #[error("repository conflict: {0}")]
    Conflict(&'static str),
    #[error("repository unavailable: {0}")]
    Unavailable(&'static str),
}

#[async_trait]
pub trait StaticAssetRepository: Send + Sync {
    async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>, RepositoryError>;
    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>, RepositoryError>;
    async fn insert_tenant(&self, tenant: Tenant) -> Result<Tenant, RepositoryError>;
    async fn insert_user(&self, user: User) -> Result<User, RepositoryError>;
    async fn insert_site(&self, site: Site) -> Result<Site, RepositoryError>;
    async fn get_site(&self, site_id: Uuid) -> Result<Option<Site>, RepositoryError>;
    async fn update_site(&self, site: Site) -> Result<Site, RepositoryError>;
}

pub struct PgStaticAssetRepository {
    pool: PgPool,
}

impl PgStaticAssetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl StaticAssetRepository for PgStaticAssetRepository {
    async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>, RepositoryError> {
        let row = sqlx::query("select id, name, slug from tenants where id = $1")
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| RepositoryError::Unavailable("postgres query failed"))?;

        Ok(row.map(|row| Tenant {
            id: row.get("id"),
            name: row.get("name"),
            slug: row.get("slug"),
        }))
    }

    async fn get_user(&self, user_id: Uuid) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query("select id, tenant_id, email, display_name from users where id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| RepositoryError::Unavailable("postgres query failed"))?;

        Ok(row.map(|row| User {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            email: row.get("email"),
            display_name: row.get("display_name"),
        }))
    }

    async fn insert_tenant(&self, tenant: Tenant) -> Result<Tenant, RepositoryError> {
        sqlx::query("insert into tenants (id, name, slug) values ($1, $2, $3)")
            .bind(tenant.id)
            .bind(&tenant.name)
            .bind(&tenant.slug)
            .execute(&self.pool)
            .await
            .map_err(|_| RepositoryError::Conflict("failed to insert tenant"))?;
        Ok(tenant)
    }

    async fn insert_user(&self, user: User) -> Result<User, RepositoryError> {
        sqlx::query(
            "insert into users (id, tenant_id, email, display_name) values ($1, $2, $3, $4)",
        )
        .bind(user.id)
        .bind(user.tenant_id)
        .bind(&user.email)
        .bind(&user.display_name)
        .execute(&self.pool)
        .await
        .map_err(|_| RepositoryError::Conflict("failed to insert user"))?;
        Ok(user)
    }

    async fn insert_site(&self, site: Site) -> Result<Site, RepositoryError> {
        sqlx::query(
            "insert into sites (id, tenant_id, owner_user_id, name, default_entry_path, file_count, total_size_bytes, storage_key, deployed_by_user_id) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(site.id)
        .bind(site.tenant_id)
        .bind(site.owner_user_id)
        .bind(&site.name)
        .bind(&site.default_entry_path)
        .bind(i64::from(site.file_count))
        .bind(site.total_size_bytes as i64)
        .bind(&site.storage_key)
        .bind(site.deployed_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(|_| RepositoryError::Conflict("failed to insert site"))?;
        Ok(site)
    }

    async fn get_site(&self, site_id: Uuid) -> Result<Option<Site>, RepositoryError> {
        let row = sqlx::query(
            "select id, tenant_id, owner_user_id, name, default_entry_path, file_count, total_size_bytes, storage_key, deployed_by_user_id from sites where id = $1",
        )
        .bind(site_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| RepositoryError::Unavailable("postgres query failed"))?;

        Ok(row.map(|row| Site {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            owner_user_id: row.get("owner_user_id"),
            name: row.get("name"),
            default_entry_path: row.get("default_entry_path"),
            file_count: row.get::<i64, _>("file_count") as u32,
            total_size_bytes: row.get::<i64, _>("total_size_bytes") as u64,
            storage_key: row.get("storage_key"),
            deployed_by_user_id: row.get("deployed_by_user_id"),
        }))
    }

    async fn update_site(&self, site: Site) -> Result<Site, RepositoryError> {
        sqlx::query(
            "update sites set tenant_id = $2, owner_user_id = $3, name = $4, default_entry_path = $5, file_count = $6, total_size_bytes = $7, storage_key = $8, deployed_by_user_id = $9 where id = $1",
        )
        .bind(site.id)
        .bind(site.tenant_id)
        .bind(site.owner_user_id)
        .bind(&site.name)
        .bind(&site.default_entry_path)
        .bind(i64::from(site.file_count))
        .bind(site.total_size_bytes as i64)
        .bind(&site.storage_key)
        .bind(site.deployed_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(|_| RepositoryError::Unavailable("failed to update site"))?;
        Ok(site)
    }
}
