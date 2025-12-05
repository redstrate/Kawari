use axum::extract::Query;
use axum::response::Html;
use axum::{Router, routing::get};
use kawari::config::get_config;
use kawari::{constants::SUPPORTED_GAME_VERSION, web_static_dir};
use minijinja::context;
use minijinja::{Environment, path_loader};
use serde::Deserialize;
use tower_http::services::ServeDir;

fn setup_default_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_loader(path_loader("resources/web/templates"));

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

// Removes the protocol bit from a URL e.g. turning http://patch-gamever.ffxiv.localhost to patch-gamever.ffxiv.localhost
// We have to do this, because LauncherTweaks expects it to be a hostname.
fn strip_out_protocol(url: &str) -> &str {
    url.split_once("://").unwrap().1
}

async fn launcher_config(Query(params): Query<Params>) -> String {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("launchertweaks.toml").unwrap();
    let game_patch_server = strip_out_protocol(&config.patch.game_server_name);
    let boot_patch_server = strip_out_protocol(&config.patch.boot_server_name);

    template
            .render(context! { launcher_url => config.launcher.server_name, enable_webview2 => params.r#type != "webview2", game_patch_server, boot_patch_server, lobby_port => config.lobby.port, lobby_host => config.lobby.server_name })
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
