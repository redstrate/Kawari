use axum::response::Html;
use axum::{Router, routing::get};
use kawari::config::get_config;
use kawari::{constants::SUPPORTED_GAME_VERSION, web_static_dir};
use minijinja::context;
use minijinja::{Environment, path_loader};
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
            .render(context! { login_server => config.login.server_name, enable_registration => config.login.enable_registration })
            .unwrap(),
    )
}

async fn setup() -> Html<String> {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("setup.html").unwrap();
    Html(
        template
            .render(context! { login_server => config.login.server_name, lobby_port => config.lobby.port, lobby_host => config.lobby.server_name, game_version => SUPPORTED_GAME_VERSION, frontier_host => config.frontier.server_name, login_host => config.login.server_name, server_url => config.web.server_name, enable_registration => config.login.enable_registration })
            .unwrap(),
    )
}

async fn auto_config() -> String {
    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("autoconfig.json").unwrap();
    template
        .render(context! {
            game_patch_server => config.patch.server_name,
            boot_patch_server => config.patch.server_name,
            login_server => config.login.server_name,
            lobby_server => config.lobby.server_name,
            lobby_port => config.lobby.port,
            frontier_server => config.frontier.server_name,
        })
        .unwrap()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/setup", get(setup))
        .route("/.well-known/xiv", get(auto_config))
        .nest_service("/static", ServeDir::new(web_static_dir!("")));

    let config = get_config();

    let addr = config.web.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
