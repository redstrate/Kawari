use axum::extract::Query;
use axum::response::Html;
use axum::{Router, routing::get};
use kawari::config::get_config;
use minijinja::Environment;
use minijinja::context;
use serde::Deserialize;

fn setup_default_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template(
        "launcher.html",
        include_str!("../../templates/launcher.html"),
    )
    .unwrap();

    env
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Params {
    rc_lang: String,
    time: String,
}

async fn root(Query(_): Query<Params>) -> Html<String> {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("launcher.html").unwrap();
    Html(
        template
            .render(context! { login_server => config.login.server_name })
            .unwrap(),
    )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/v700/index.html", get(root));

    let config = get_config();

    let addr = config.launcher.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
