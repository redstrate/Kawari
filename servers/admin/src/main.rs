use axum::response::{Html, Redirect};
use axum::routing::post;
use axum::{Router, extract::Form, routing::get};
use kawari::common::{BasicCharacterData, User};
use kawari::config::get_config;
use kawari::ipc::kawari::{CustomIpcData, CustomIpcSegment};
use kawari::packet::send_custom_world_packet;
use kawari::web_static_dir;
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
    let template = environment.get_template("admin_general.html").unwrap();
    Html(template.render(context! { config }).unwrap())
}

async fn users() -> Html<String> {
    let environment = setup_default_environment();
    let template = environment.get_template("admin_users.html").unwrap();
    let config = get_config();

    let Ok(mut login_reply) =
        ureq::get(&*format!("{}/_private/users", config.login.server_name)).call()
    else {
        // TODO: add a better error message here
        tracing::warn!("Failed to contact login server, is it running?");
        return Html(template.render(context! {}).unwrap());
    };

    let Ok(body) = login_reply.body_mut().read_to_string() else {
        // TODO: add a better error message here
        tracing::warn!("Failed to contact login server, is it running?");
        return Html(template.render(context! {}).unwrap());
    };

    let users: Option<Vec<User>> = serde_json::from_str(&body).ok();

    Html(template.render(context! { users }).unwrap())
}

async fn characters() -> Html<String> {
    let environment = setup_default_environment();
    let template = environment.get_template("admin_characters.html").unwrap();

    let ipc_segment = CustomIpcSegment::new(CustomIpcData::RequestFullCharacterList {});

    if let Some(response) = send_custom_world_packet(ipc_segment).await
        && let CustomIpcData::FullCharacterListResponse { json } = response.data
    {
        let characters: Option<Vec<BasicCharacterData>> = serde_json::from_str(&json).ok();
        Html(template.render(context! { characters }).unwrap())
    } else {
        // error out better than this
        Html(template.render(context! {}).unwrap())
    }
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Input {
    worlds_open: Option<String>,
    login_open: Option<String>,
    festival0: Option<u16>,
    festival1: Option<u16>,
    festival2: Option<u16>,
    festival3: Option<u16>,
    world: Option<u16>,
    login_message: Option<String>,
}

async fn apply(Form(input): Form<Input>) -> Redirect {
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

    config.world.active_festivals = [
        input.festival0.unwrap_or(0),
        input.festival1.unwrap_or(1),
        input.festival2.unwrap_or(2),
        input.festival3.unwrap_or(3),
    ];

    if let Some(world) = input.world {
        config.world.world_id = world;
    }

    if let Some(login_message) = input.login_message {
        config.world.login_message = login_message;
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
        .route("/characters", get(characters))
        .nest_service("/static", ServeDir::new(web_static_dir!("")));

    let config = get_config();

    let addr = config.admin.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
