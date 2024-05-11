use std::net::SocketAddr;

use axum::{
    Json,
    Router, routing::get,
    extract::Form
};
use serde::{Deserialize, Serialize};
use axum::response::{Html, Redirect};
use axum::routing::post;
use kawari::config::{Config, get_config};
use minijinja::{Environment, context};
use kawari::setup_default_environment;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn root() -> Html<String> {
    tracing::info!("Requesting gate status...");

    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("admin.html").unwrap();
    Html(template.render(context! { gate_open => config.gate_open }).unwrap())
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