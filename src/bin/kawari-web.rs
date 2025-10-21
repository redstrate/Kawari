use axum::extract::Query;
use axum::response::Html;
use axum::{Router, routing::get};
use kawari::config::get_config;
use kawari::{constants::SUPPORTED_GAME_VERSION, web_static_dir, web_templates_dir};
use minijinja::Environment;
use minijinja::context;
use serde::Deserialize;
use tower_http::services::ServeDir;

fn setup_default_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template_owned(
        "layout.html",
        std::fs::read_to_string(web_templates_dir!("layout.html"))
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "web_base.html",
        std::fs::read_to_string(web_templates_dir!("web_base.html"))
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "web.html",
        std::fs::read_to_string(web_templates_dir!("web.html")).expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "worldstatus.html",
        std::fs::read_to_string(web_templates_dir!("worldstatus.html"))
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "account.html",
        std::fs::read_to_string(web_templates_dir!("account.html"))
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "setup.html",
        std::fs::read_to_string(web_templates_dir!("setup.html"))
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "launchertweaks.toml",
        std::fs::read_to_string(web_templates_dir!("launchertweaks.toml"))
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "autoconfig.json",
        std::fs::read_to_string(web_templates_dir!("autoconfig.json"))
            .expect("Failed to find template!"),
    )
    .unwrap();

    env
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
            .render(context! { login_server => config.login.server_name, lobby_port => config.lobby.port, lobby_host => config.lobby.server_name, game_version => SUPPORTED_GAME_VERSION, frontier_host => config.frontier.server_name, login_host => config.login.server_name, server_url => config.web.server_name })
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
            .render(context! { launcher_url => config.launcher.server_name, enable_webview2 => params.r#type != "webview2", game_patch_server => config.patch.game_server_name, boot_patch_server => config.patch.boot_server_name, lobby_port => config.lobby.port, lobby_host => config.lobby.server_name })
            .unwrap()
}

async fn auto_config() -> String {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("autoconfig.json").unwrap();
    template
        .render(context! {
            game_patch_server => config.patch.game_server_name,
            boot_patch_server => config.patch.boot_server_name,
            login_server => config.login.server_name,
            lobby_server => config.lobby.server_name,
            lobby_port => config.lobby.port,
            frontier_server => config.frontier.server_name,
            datacenter_travel_server => config.datacenter_travel.server_name,
        })
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
        .route("/.well-known/xiv", get(auto_config))
        .nest_service("/static", ServeDir::new(web_static_dir!("")));

    let config = get_config();

    let addr = config.web.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
