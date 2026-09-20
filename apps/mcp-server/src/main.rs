use std::sync::Arc;

use application::{
    ActivateVersionInput, ApplicationError, RegisterSiteInput, StaticAssetService,
    UploadArtifactInput,
};
use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use common::APP_NAME;
use domain::DomainError;
use observability::init_tracing;
use serde::{Deserialize, Serialize};
use settings::ServerSettings;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    service: Arc<StaticAssetService>,
}

#[derive(Debug, Deserialize)]
struct ActivateArtifactRequest {
    version_id: Uuid,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let server_settings = ServerSettings::from_env("MCP_SERVER_ADDR", "127.0.0.1:3000");
    let state = AppState {
        service: Arc::new(StaticAssetService::new()),
    };

    info!(app = APP_NAME, addr = %server_settings.addr, "starting mcp server");

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/sites", post(register_site))
        .route("/artifacts", post(upload_artifact))
        .route("/sites/{site_id}/activate", post(activate_artifact))
        .route("/sites/{site_id}/runtime", get(get_runtime_info))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(&server_settings.addr)
        .await
        .expect("listener bind must succeed");

    axum::serve(listener, app).await.expect("server must serve");
}

async fn healthz() -> &'static str {
    "ok"
}

async fn register_site(
    State(state): State<AppState>,
    Json(input): Json<RegisterSiteInput>,
) -> impl IntoResponse {
    match state.service.register_site(input) {
        Ok(site) => (StatusCode::CREATED, Json(site)).into_response(),
        Err(error) => application_error_response(error),
    }
}

async fn upload_artifact(
    State(state): State<AppState>,
    Json(input): Json<UploadArtifactInput>,
) -> impl IntoResponse {
    match state.service.upload_artifact(input) {
        Ok(version) => (StatusCode::CREATED, Json(version)).into_response(),
        Err(error) => application_error_response(error),
    }
}

async fn activate_artifact(
    State(state): State<AppState>,
    AxumPath(site_id): AxumPath<Uuid>,
    Json(input): Json<ActivateArtifactRequest>,
) -> impl IntoResponse {
    // 这里暴露的是“激活站点版本”这个高层动作，而不是让 agent 自己拼接内部状态变更。
    match state.service.activate_version(ActivateVersionInput {
        site_id,
        version_id: input.version_id,
    }) {
        Ok(runtime) => (StatusCode::OK, Json(runtime)).into_response(),
        Err(error) => application_error_response(error),
    }
}

async fn get_runtime_info(
    State(state): State<AppState>,
    AxumPath(site_id): AxumPath<Uuid>,
) -> impl IntoResponse {
    match state.service.get_runtime_info(site_id) {
        Ok(runtime) => (StatusCode::OK, Json(runtime)).into_response(),
        Err(error) => application_error_response(error),
    }
}

fn application_error_response(error: ApplicationError) -> axum::response::Response {
    let status = match &error {
        ApplicationError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        ApplicationError::Domain(
            DomainError::SiteNotFound(_)
            | DomainError::ArtifactVersionNotFound(_),
        ) => StatusCode::NOT_FOUND,
        ApplicationError::Domain(
            DomainError::VersionSiteMismatch { .. } | DomainError::DuplicateArtifactLabel { .. },
        ) => StatusCode::CONFLICT,
        ApplicationError::StatePoisoned => StatusCode::INTERNAL_SERVER_ERROR,
    };

    if status == StatusCode::INTERNAL_SERVER_ERROR {
        error!(%error, "request failed");
    }

    (
        status,
        Json(ApiErrorBody {
            error: error.to_string(),
        }),
    )
        .into_response()
}
