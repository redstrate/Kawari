use axum::response::{Html, Redirect};
use axum::routing::post;
use axum::{Router, extract::Form, routing::get};
use kawari::common::{BasicCharacterData, BasicServiceAccountData, User};
use kawari::config::get_config;
use kawari::festivals::festival_list;
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
    let festival_list = festival_list();

    let environment = setup_default_environment();
    let template = environment.get_template("admin_general.html").unwrap();
    Html(template.render(context! { config, festival_list }).unwrap())
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

async fn service_accounts() -> Html<String> {
    let environment = setup_default_environment();
    let template = environment
        .get_template("admin_serviceaccounts.html")
        .unwrap();

    let config = get_config();
    let Ok(mut login_reply) = ureq::get(&*format!(
        "{}/_private/all_service_accounts",
        config.login.server_name
    ))
    .call() else {
        // TODO: add a better error message here
        tracing::warn!("Failed to contact login server, is it running?");
        return Html(template.render(context! {}).unwrap());
    };

    let Ok(body) = login_reply.body_mut().read_to_string() else {
        // TODO: add a better error message here
        tracing::warn!("Failed to contact login server, is it running?");
        return Html(template.render(context! {}).unwrap());
    };

    let service_accounts: Option<Vec<BasicServiceAccountData>> = serde_json::from_str(&body).ok();

    Html(template.render(context! { service_accounts }).unwrap())
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
    festival4: Option<u16>,
    festival5: Option<u16>,
    festival6: Option<u16>,
    festival7: Option<u16>,
    world: Option<u16>,
    login_message: Option<String>,
    enable_registration: Option<String>,
}

async fn apply(Form(input): Form<Input>) -> Redirect {
    let mut config = get_config();

    if let Some(value) = input.worlds_open {
        config.frontier.worlds_open = value == "on";
    } else {
        config.frontier.worlds_open = false;
    }

    if let Some(value) = input.login_open {
        config.frontier.login_open = value == "on";
    } else {
        config.frontier.login_open = false;
    }

    if let Some(value) = input.enable_registration {
        config.login.enable_registration = value == "on";
    } else {
        config.login.enable_registration = false;
    }

    config.world.active_festivals = [
        input.festival0.unwrap_or(0),
        input.festival1.unwrap_or(0),
        input.festival2.unwrap_or(0),
        input.festival3.unwrap_or(0),
        input.festival4.unwrap_or(0),
        input.festival5.unwrap_or(0),
        input.festival6.unwrap_or(0),
        input.festival7.unwrap_or(0),
    ];

    if let Some(world) = input.world {
        config.world.world_id = world;
    }

    if let Some(login_message) = input.login_message {
        config.world.login_message = login_message;
    }

    serde_yaml_ng::to_writer(&std::fs::File::create("config.yaml").unwrap(), &config)
        .expect("TODO: panic message");

    // Reload active festivals on the World server
    // (There is no response.)
    let _ = send_custom_world_packet(CustomIpcSegment::new(CustomIpcData::ReloadFestivals)).await;

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
        .route("/service_accounts", get(service_accounts))
        .nest_service("/static", ServeDir::new(web_static_dir!("")));

    let config = get_config();

    let addr = config.admin.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
