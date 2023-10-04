use std::net::SocketAddr;

use axum::{
    Json,
    Router, routing::get,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn get_gate_status() -> Json<GateStatus> {
    tracing::info!("Requesting gate status...");

    Json(GateStatus {
        status: 1
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/worldStatus/gate_status.json", get(get_gate_status));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Frontier server started on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}