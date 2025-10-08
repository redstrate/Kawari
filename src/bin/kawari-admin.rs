use axum::response::{Html, Redirect};
use axum::routing::post;
use axum::{Router, extract::Form, routing::get};
use kawari::config::get_config;
use kawari::login::User;
use kawari::web_static_dir;
use minijinja::Environment;
use minijinja::context;
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

fn setup_default_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template_owned(
        "layout.html",
        std::fs::read_to_string("resources/web/templates/layout.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "admin_general.html",
        std::fs::read_to_string("resources/web/templates/admin_general.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "admin_base.html",
        std::fs::read_to_string("resources/web/templates/admin_base.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "admin_users.html",
        std::fs::read_to_string("resources/web/templates/admin_users.html")
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
    tracing::info!("Requesting gate status...");

    let config = get_config();

    let environment = setup_default_environment();
    let template = environment.get_template("admin_general.html").unwrap();
    Html(template.render(context! { worlds_open => config.frontier.worlds_open, login_open => config.frontier.login_open, boot_patch_location => config.patch.patches_location }).unwrap())
}

async fn users() -> Html<String> {
    let environment = setup_default_environment();
    let template = environment.get_template("admin_users.html").unwrap();
    let config = get_config();

    let Ok(login_reply) =
        reqwest::get(&*format!("{}/_private/users", config.login.server_name)).await
    else {
        // TODO: add a better error message here
        tracing::warn!("Failed to contact login server, is it running?");
        return Html(template.render(context! {}).unwrap());
    };

    let Ok(body) = login_reply.text().await else {
        // TODO: add a better error message here
        tracing::warn!("Failed to contact login server, is it running?");
        return Html(template.render(context! {}).unwrap());
    };

    let users: Option<Vec<User>> = serde_json::from_str(&body).ok();

    Html(template.render(context! { users => users }).unwrap())
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
        config.frontier.worlds_open = gate_open == "on";
    } else {
        config.frontier.worlds_open = false;
    }

    if let Some(gate_open) = input.login_open {
        config.frontier.login_open = gate_open == "on";
    } else {
        config.frontier.login_open = false;
    }

    if let Some(boot_patch_location) = input.boot_patch_location {
        config.patch.patches_location = boot_patch_location;
    }

    serde_yaml_ng::to_writer(&std::fs::File::create("config.yaml").unwrap(), &config)
        .expect("TODO: panic message");

    Redirect::to("/")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/apply", post(apply))
        .route("/users", get(users))
        .nest_service("/static", ServeDir::new(web_static_dir!("")));

    let config = get_config();

    let addr = config.admin.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
