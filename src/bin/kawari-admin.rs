use std::net::SocketAddr;

use axum::response::{Html, Redirect};
use axum::routing::post;
use axum::{Router, extract::Form, routing::get};
use kawari::config::get_config;
use kawari::setup_default_environment;
use minijinja::context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn root() -> Html<String> {
    tracing::info!("Requesting gate status...");

    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("admin.html").unwrap();
    Html(template.render(context! { worlds_open => config.worlds_open, login_open => config.login_open, boot_patch_location => config.boot_patches_location }).unwrap())
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Input {
    worlds_open: Option<String>,
    login_open: Option<String>,
    boot_patch_location: Option<String>,
}

async fn apply(Form(input): Form<Input>) -> Redirect {
    tracing::info!("Apply config changes...");

    let mut config = get_config();

    if let Some(gate_open) = input.worlds_open {
        config.worlds_open = gate_open == "on";
    } else {
        config.worlds_open = false;
    }

    if let Some(gate_open) = input.login_open {
        config.login_open = gate_open == "on";
    } else {
        config.login_open = false;
    }

    if let Some(boot_patch_location) = input.boot_patch_location {
        config.boot_patches_location = boot_patch_location;
    }

    serde_json::to_writer(&std::fs::File::create("config.json").unwrap(), &config)
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
