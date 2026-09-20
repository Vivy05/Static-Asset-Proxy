use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use application::StaticAssetService;
use axum::{
    Router,
    body::Body,
    extract::{Path as AxumPath, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use common::APP_NAME;
use domain::DomainError;
use observability::init_tracing;
use settings::{ServerSettings, StorageSettings};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    service: Arc<StaticAssetService>,
    storage_root: PathBuf,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let server_settings = ServerSettings::from_env("GATEWAY_ADDR", "127.0.0.1:4000");
    let storage_settings = StorageSettings::from_env("STATIC_ROOT", "./data/static");
    let state = AppState {
        service: Arc::new(StaticAssetService::new()),
        storage_root: PathBuf::from(storage_settings.static_root),
    };

    info!(
        app = APP_NAME,
        addr = %server_settings.addr,
        storage_root = %state.storage_root.display(),
        "starting gateway"
    );

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/sites/{site_id}", get(serve_default_entry))
        .route("/sites/{site_id}/*asset_path", get(serve_site_asset))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(&server_settings.addr)
        .await
        .expect("listener bind must succeed");

    axum::serve(listener, app).await.expect("server must serve");
}

async fn healthz() -> &'static str {
    "ok"
}

async fn serve_default_entry(
    State(state): State<AppState>,
    AxumPath(site_id): AxumPath<Uuid>,
) -> Response {
    serve_site_path(state, site_id, None)
}

async fn serve_site_asset(
    State(state): State<AppState>,
    AxumPath((site_id, asset_path)): AxumPath<(Uuid, String)>,
) -> Response {
    serve_site_path(state, site_id, Some(asset_path))
}

fn serve_site_path(state: AppState, site_id: Uuid, request_path: Option<String>) -> Response {
    let runtime = match state.service.get_runtime_info(site_id) {
        Ok(runtime) => runtime,
        Err(application::ApplicationError::Domain(DomainError::SiteNotFound(_))) => {
            return (StatusCode::NOT_FOUND, "site not found").into_response();
        }
        Err(error) => {
            error!(site_id = %site_id, %error, "failed to load runtime info");
            return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load site runtime")
                .into_response();
        }
    };

    // gateway 只服务已经被激活的版本，不会根据上传目录自行猜测当前对外流量应该落到哪个版本。
    let active_version = match runtime.active_version {
        Some(version) => version,
        None => return (StatusCode::CONFLICT, "site has no active deployment").into_response(),
    };

    let resolved = match resolve_asset_path(
        &state.storage_root,
        &active_version.storage_key,
        &active_version.entry_file,
        request_path.as_deref(),
    ) {
        Ok(path) => path,
        Err(status) => return status.into_response(),
    };

    match std::fs::read(&resolved) {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&resolved).first_or_octet_stream();
            let mut response = Response::new(Body::from(bytes));
            *response.status_mut() = StatusCode::OK;
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=60"),
            );
            response
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            (StatusCode::NOT_FOUND, "asset not found").into_response()
        }
        Err(err) => {
            error!(
                site_id = %site_id,
                asset_path = %resolved.display(),
                %err,
                "failed to read asset"
            );
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read asset").into_response()
        }
    }
}

fn resolve_asset_path(
    storage_root: &Path,
    storage_key: &str,
    default_entry_path: &str,
    request_path: Option<&str>,
) -> Result<PathBuf, StatusCode> {
    let relative_asset = match request_path {
        Some(path) if !path.trim().is_empty() => path,
        _ => default_entry_path,
    };
    let mut full_path = storage_root.join(storage_key);

    // 这里只允许在当前版本自己的存储前缀下读文件。
    // 拒绝 `..` 这类父级路径，避免恶意请求逃逸到其他版本目录或任意文件。
    for segment in Path::new(relative_asset).components() {
        match segment {
            Component::Normal(part) => full_path.push(part),
            Component::CurDir => {}
            Component::RootDir => {}
            Component::ParentDir | Component::Prefix(_) => return Err(StatusCode::BAD_REQUEST),
        }
    }

    Ok(full_path)
}
