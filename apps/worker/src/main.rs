use common::APP_NAME;
use observability::init_tracing;
use tracing::info;

#[tokio::main]
async fn main() {
    init_tracing();
    info!(app = APP_NAME, "starting worker stage 0");
}
