use axum::extract::Query;
use axum::response::Html;
use axum::{Router, routing::get};
use kawari::config::get_config;
use minijinja::Environment;
use minijinja::context;
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

fn setup_default_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template_owned(
        "layout.html",
        std::fs::read_to_string("resources/templates/layout.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "web_base.html",
        std::fs::read_to_string("resources/templates/web_base.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "web.html",
        std::fs::read_to_string("resources/templates/web.html").expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "worldstatus.html",
        std::fs::read_to_string("resources/templates/worldstatus.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "account.html",
        std::fs::read_to_string("resources/templates/account.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "setup.html",
        std::fs::read_to_string("resources/templates/setup.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "launchertweaks.toml",
        std::fs::read_to_string("resources/templates/launchertweaks.toml")
            .expect("Failed to find template!"),
    )
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
            .render(context! { login_server => config.login.server_name, login_open => config.frontier.login_open, worlds_open => config.frontier.worlds_open })
            .unwrap(),
    )
}

async fn setup() -> Html<String> {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("setup.html").unwrap();
    Html(
        template
            .render(context! { login_server => config.login.server_name })
            .unwrap(),
    )
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Params {
    r#type: String,
}

async fn launcher_config(Query(params): Query<Params>) -> String {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("launchertweaks.toml").unwrap();
    template
            .render(context! { launcher_url => config.launcher.server_name, enable_webview2 => params.r#type != "webview2", game_patch_server => config.patch.game_server_name, boot_patch_server => config.patch.boot_server_name, lobby_port => config.lobby.port })
            .unwrap()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/worldstatus", get(world_status))
        .route("/setup", get(setup))
        .route("/launcherconfig", get(launcher_config))
        .nest_service("/static", ServeDir::new("resources/static"));

    let config = get_config();

    let addr = config.web.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
