use axum::{Router, routing::get};
use common::APP_NAME;
use observability::init_tracing;
use settings::ServerSettings;
use tracing::info;

#[tokio::main]
async fn main() {
    init_tracing();

    let settings = ServerSettings::new("127.0.0.1:3000");
    info!(app = APP_NAME, addr = %settings.addr, "starting mcp server");

    let app = Router::new().route("/healthz", get(healthz));
    let listener = tokio::net::TcpListener::bind(&settings.addr)
        .await
        .expect("listener bind must succeed");

    axum::serve(listener, app).await.expect("server must serve");
}

async fn healthz() -> &'static str {
    "ok"
}
