use std::net::SocketAddr;

use axum::response::Html;
use axum::{Router, routing::get};
use kawari::config::get_config;
use kawari::setup_default_environment;
use minijinja::context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn root() -> Html<String> {
    let environment = setup_default_environment();
    let template = environment.get_template("web.html").unwrap();
    Html(template.render(context! {}).unwrap())
}

async fn login() -> Html<String> {
    let environment = setup_default_environment();
    let template = environment.get_template("login.html").unwrap();
    Html(template.render(context! {}).unwrap())
}

async fn register() -> Html<String> {
    let environment = setup_default_environment();
    let template = environment.get_template("register.html").unwrap();
    Html(template.render(context! {}).unwrap())
}

async fn world_status() -> Html<String> {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("worldstatus.html").unwrap();
    Html(
        template
            .render(context! { login_open => config.login_open, worlds_open => config.worlds_open })
            .unwrap(),
    )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/login", get(login))
        .route("/register", get(register))
        .route("/worldstatus", get(world_status));

    let addr = SocketAddr::from(([127, 0, 0, 1], 5801));
    tracing::info!("Web server started on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
