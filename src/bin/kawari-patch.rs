use std::net::SocketAddr;

use axum::{Form, Json, Router, routing::get};
use axum::extract::Query;
use axum::response::Html;
use axum::routing::post;
use serde::{Deserialize, Serialize};
use kawari::config::{Config, get_config};
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::http::{HeaderMap, StatusCode};

async fn verify_session(Path((platform, game_version, sid)): Path<(String, String, String)>) -> impl IntoResponse {
    let config = get_config();
    if !config.supports_platform(&platform) {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut headers = HeaderMap::new();
    headers.insert("X-Patch-Unique-Id", sid.parse().unwrap());

    (headers).into_response()
}

async fn verify_boot(Path((platform, boot_version)): Path<(String, String)>) -> impl IntoResponse {
    let config = get_config();
    if !config.supports_platform(&platform) {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut headers = HeaderMap::new();
    (headers).into_response()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/http/:platform/ffxivneo_release_game/:game_version/:sid", post(verify_session))
        .route("/http/:platform/ffxivneo_release_boot/:boot_version", get(verify_boot));

    let addr = SocketAddr::from(([127, 0, 0, 1], 6900));
    tracing::info!("Patch server started on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}