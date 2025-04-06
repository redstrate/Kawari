use axum::response::Html;
use axum::{Router, routing::get};
use kawari::config::get_config;
use minijinja::Environment;
use minijinja::context;
use serde::{Deserialize, Serialize};

fn setup_default_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("web.html", include_str!("../../templates/web.html"))
        .unwrap();
    env.add_template(
        "worldstatus.html",
        include_str!("../../templates/worldstatus.html"),
    )
    .unwrap();
    env.add_template("account.html", include_str!("../../templates/account.html"))
        .unwrap();

    env
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn root() -> Html<String> {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("web.html").unwrap();
    Html(
        template
            .render(context! { login_server => config.login.server_name })
            .unwrap(),
    )
}

async fn world_status() -> Html<String> {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("worldstatus.html").unwrap();
    Html(
        template
            .render(context! { login_open => config.frontier.login_open, worlds_open => config.frontier.worlds_open })
            .unwrap(),
    )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/worldstatus", get(world_status));

    let config = get_config();

    let addr = config.web.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
