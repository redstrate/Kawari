use std::net::SocketAddr;

use axum::{
    Json,
    Router, routing::get,
};
use serde::{Deserialize, Serialize};
use kawari::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn get_gate_status() -> Json<GateStatus> {
    tracing::info!("Requesting gate status...");

    let mut is_open = 0;

    // read config
    if let Ok(data) = std::fs::read_to_string("config.json") {
        let config: Config = serde_json::from_str(&data).expect("Failed to parse");

        if config.gate_open {
            is_open = 1;
        }
    }

    Json(GateStatus {
        status: is_open
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/worldStatus/gate_status.json", get(get_gate_status));

    let addr = SocketAddr::from(([127, 0, 0, 1], 5857));
    tracing::info!("Frontier server started on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}