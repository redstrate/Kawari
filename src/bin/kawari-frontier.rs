use std::net::SocketAddr;

use axum::{Json, Router, routing::get};
use kawari::config::get_config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn get_login_status() -> Json<GateStatus> {
    tracing::info!("Requesting login status...");

    let config = get_config();
    Json(GateStatus {
        status: config.login_open.into(),
    })
}

async fn get_world_status() -> Json<GateStatus> {
    tracing::info!("Requesting world status...");

    let config = get_config();
    Json(GateStatus {
        status: config.worlds_open.into(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Banner {
    link: String,
    lsb_banner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NewsItem {
    date: String,
    id: String,
    tag: String,
    title: String,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Headline {
    banner: Vec<Banner>,
    news: Vec<NewsItem>,
    pinned: Vec<NewsItem>,
    topics: Vec<NewsItem>,
}

async fn get_headline() -> Json<Headline> {
    tracing::info!("Requesting headline...");

    Json(Headline {
        banner: vec![],
        news: vec![NewsItem {
            date: "".to_string(),
            id: "".to_string(),
            tag: "".to_string(),
            title: "You are connected to Kawari".to_string(),
            url: "https://github.com/redstrate/Kawari".to_string(),
        }],
        pinned: vec![],
        topics: vec![],
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/worldStatus/gate_status.json", get(get_world_status))
        .route("/worldStatus/login_status.json", get(get_login_status))
        .route("/news/headline.json", get(get_headline));

    let addr = SocketAddr::from(([127, 0, 0, 1], 5857));
    tracing::info!("Frontier server started on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
