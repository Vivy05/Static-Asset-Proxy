use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use domain::{ArtifactStatus, ArtifactVersion, DomainError, RuntimeInfo, Site};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSiteInput {
    pub name: String,
    pub default_entry_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadArtifactInput {
    pub site_id: Uuid,
    pub label: String,
    pub storage_key: String,
    pub entry_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateVersionInput {
    pub site_id: Uuid,
    pub version_id: Uuid,
}

#[derive(Debug, Clone, Default)]
pub struct StaticAssetService {
    inner: Arc<RwLock<InnerState>>,
}

#[derive(Debug, Default)]
struct InnerState {
    sites: HashMap<Uuid, Site>,
    versions: HashMap<Uuid, ArtifactVersion>,
}

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("state lock poisoned")]
    StatePoisoned,
    #[error(transparent)]
    Domain(#[from] DomainError),
}

impl StaticAssetService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_site(&self, input: RegisterSiteInput) -> Result<Site, ApplicationError> {
        if input.name.trim().is_empty() {
            return Err(ApplicationError::InvalidInput("site name cannot be empty"));
        }

        let site = Site {
            id: Uuid::new_v4(),
            name: input.name,
            default_entry_path: input.default_entry_path.unwrap_or_else(|| "/".to_string()),
            active_version_id: None,
        };

        let mut state = self
            .inner
            .write()
            .map_err(|_| ApplicationError::StatePoisoned)?;
        state.sites.insert(site.id, site.clone());
        Ok(site)
    }

    pub fn upload_artifact(
        &self,
        input: UploadArtifactInput,
    ) -> Result<ArtifactVersion, ApplicationError> {
        if input.label.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "artifact label cannot be empty",
            ));
        }
        if input.storage_key.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "storage key cannot be empty",
            ));
        }

        let mut state = self
            .inner
            .write()
            .map_err(|_| ApplicationError::StatePoisoned)?;
        if !state.sites.contains_key(&input.site_id) {
            return Err(DomainError::SiteNotFound(input.site_id).into());
        }

        let version = ArtifactVersion {
            id: Uuid::new_v4(),
            site_id: input.site_id,
            label: input.label,
            storage_key: input.storage_key,
            entry_file: input.entry_file.unwrap_or_else(|| "index.html".to_string()),
            status: ArtifactStatus::Uploaded,
        };

        state.versions.insert(version.id, version.clone());
        Ok(version)
    }

    pub fn activate_version(
        &self,
        input: ActivateVersionInput,
    ) -> Result<RuntimeInfo, ApplicationError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| ApplicationError::StatePoisoned)?;
        let version = state
            .versions
            .get(&input.version_id)
            .cloned()
            .ok_or(DomainError::ArtifactVersionNotFound(input.version_id))?;

        if version.site_id != input.site_id {
            return Err(DomainError::VersionSiteMismatch {
                site_id: input.site_id,
                version_id: input.version_id,
            }
            .into());
        }

        let site = state
            .sites
            .get_mut(&input.site_id)
            .ok_or(DomainError::SiteNotFound(input.site_id))?;

        for item in state.versions.values_mut() {
            if item.site_id == input.site_id && item.status == ArtifactStatus::Active {
                item.status = ArtifactStatus::Superseded;
            }
        }

        if let Some(item) = state.versions.get_mut(&input.version_id) {
            item.status = ArtifactStatus::Active;
        }

        site.active_version_id = Some(input.version_id);
        build_runtime_info(&state, input.site_id)
    }

    pub fn get_runtime_info(&self, site_id: Uuid) -> Result<RuntimeInfo, ApplicationError> {
        let state = self
            .inner
            .read()
            .map_err(|_| ApplicationError::StatePoisoned)?;
        build_runtime_info(&state, site_id)
    }

    pub fn list_site_versions(
        &self,
        site_id: Uuid,
    ) -> Result<Vec<ArtifactVersion>, ApplicationError> {
        let state = self
            .inner
            .read()
            .map_err(|_| ApplicationError::StatePoisoned)?;
        let runtime = build_runtime_info(&state, site_id)?;
        Ok(runtime.versions)
    }
}

fn build_runtime_info(state: &InnerState, site_id: Uuid) -> Result<RuntimeInfo, ApplicationError> {
    let site = state
        .sites
        .get(&site_id)
        .cloned()
        .ok_or(DomainError::SiteNotFound(site_id))?;

    let mut versions = state
        .versions
        .values()
        .filter(|version| version.site_id == site_id)
        .cloned()
        .collect::<Vec<_>>();
    versions.sort_by_key(|version| version.label.clone());

    let active_version = site
        .active_version_id
        .and_then(|active_id| state.versions.get(&active_id).cloned());

    Ok(RuntimeInfo {
        site,
        active_version,
        versions,
    })
}

#[cfg(test)]
mod tests {
    use super::{ActivateVersionInput, RegisterSiteInput, StaticAssetService, UploadArtifactInput};
    use domain::ArtifactStatus;

    #[test]
    fn activates_uploaded_version_and_exposes_runtime_info() {
        let service = StaticAssetService::new();
        let site = service
            .register_site(RegisterSiteInput {
                name: "docs".to_string(),
                default_entry_path: Some("/".to_string()),
            })
            .expect("site registration should succeed");

        let version = service
            .upload_artifact(UploadArtifactInput {
                site_id: site.id,
                label: "v1".to_string(),
                storage_key: "site/docs/v1".to_string(),
                entry_file: Some("index.html".to_string()),
            })
            .expect("artifact upload should succeed");

        let runtime = service
            .activate_version(ActivateVersionInput {
                site_id: site.id,
                version_id: version.id,
            })
            .expect("activation should succeed");

        assert_eq!(runtime.site.active_version_id, Some(version.id));
        assert_eq!(
            runtime
                .active_version
                .expect("runtime should expose active version")
                .status,
            ArtifactStatus::Active
        );
    }
}
