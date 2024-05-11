use std::net::SocketAddr;

use axum::{
    Json,
    Router, routing::get,
    extract::Form
};
use serde::{Deserialize, Serialize};
use axum::response::{Html, Redirect};
use axum::routing::post;
use kawari::config::Config;

fn get_config() -> Config {
    if let Ok(data) = std::fs::read_to_string("config.json") {
        serde_json::from_str(&data).expect("Failed to parse")
    } else {
        Config::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn root() -> Html<String> {
    tracing::info!("Requesting gate status...");

    let config = get_config();

    Html(format!("<p>Gate open:{}</p><form action='apply' method='post'><input type='checkbox' id='gate_open' name='gate_open' checked /><button type='submit'>Apply</button></form>", config.gate_open))
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Input {
    gate_open: Option<String>,
}

async fn apply(Form(input): Form<Input>) -> Redirect {
    tracing::info!("Apply config changes...");

    let mut config = get_config();

    if let Some(gate_open) = input.gate_open {
        config.gate_open = gate_open == "on";
    } else {
        config.gate_open = false;
    }

    serde_json::to_writer(
        &std::fs::File::create("config.json").unwrap(),
        &config,
    )
    .expect("TODO: panic message");

    Redirect::to("/")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/apply", post(apply));

    let addr = SocketAddr::from(([127, 0, 0, 1], 5800));
    tracing::info!("Admin server started on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}