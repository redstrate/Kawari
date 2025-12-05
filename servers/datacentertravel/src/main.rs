use axum::{
    Router,
    http::{StatusCode, Uri},
};
use kawari::config::get_config;

async fn fallback(uri: Uri) -> (StatusCode, String) {
    tracing::warn!("{}", uri);
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().fallback(fallback);

    let config = get_config();

    let addr = config.datacenter_travel.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
