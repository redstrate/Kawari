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

async fn get_login_status() -> Json<GateStatus> {
    tracing::info!("Requesting gate status...");

    let mut is_open = 0;

    // read config
    if let Ok(data) = std::fs::read_to_string("config.json") {
        let config: Config = serde_json::from_str(&data).expect("Failed to parse");

        if config.login_open {
            is_open = 1;
        }
    }

    Json(GateStatus {
        status: is_open
    })
}

async fn get_world_status() -> Json<GateStatus> {
    tracing::info!("Requesting gate status...");

    let mut is_open = 0;

    // read config
    if let Ok(data) = std::fs::read_to_string("config.json") {
        let config: Config = serde_json::from_str(&data).expect("Failed to parse");

        if config.worlds_open {
            is_open = 1;
        }
    }

    Json(GateStatus {
        status: is_open
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Banner {
    link: String,
    lsb_banner: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NewsItem {
    date: String,
    id: String,
    tag: String,
    title: String,
    url: String
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
            title: "Test News Item".to_string(),
            url: "".to_string(),
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