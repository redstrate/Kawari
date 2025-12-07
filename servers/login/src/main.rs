use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Multipart, Query, State};
use axum::http::Response;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::post;
use axum::{Form, Router, routing::get};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, Expiration};
use kawari::common::{ACCOUNT_MANAGEMENT_SERVICE, GAME_SERVICE};
use kawari::config::get_config;
use kawari::ipc::kawari::{CustomIpcData, CustomIpcSegment};
use kawari::packet::send_custom_world_packet;
use kawari::web_static_dir;
use kawari_login::{LoginDatabase, LoginError};
use minijinja::{Environment, context, path_loader};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

fn setup_default_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_loader(path_loader("resources/web/templates"));

    env
}

#[derive(Clone)]
struct LoginServerState {
    database: Arc<LoginDatabase>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Params {
    lng: String,
    rgn: String,
    isft: String,
    cssmode: String,
    isnew: String,
    launchver: String,
}

async fn top(Query(_): Query<Params>) -> Html<&'static str> {
    Html(
        "\r\n<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//EN\" \"http://www.w3.org/TR/html4/loose.dtd\">\r\n<html lang=en-GB id=gb>\r\n<head>\r\n<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\" /></head><form action=\"login.send\" method=\"post\" name=\"mainForm\">\r\n\t\r\n\t\t\r\n\t\t<input type=\"hidden\" name=\"_STORED_\" value=\"42f06e5f4194001a9ad61c8481f435e8b9eac79242f9221d463aa492ab2b3373655adadff3e72dd16a798ee8a222c519848743c97084f1af71854f06050a1f2813e5c3aaf66e5f0ef24dc18588a8cf06758992e42035f7e4f99f85c8b6082200dcabc6a37c7f76ce542eefc1f1798da5e23fd4b46ed17489de5eb8e8a222c5198487433bff5f3433c061ded661b3f33b5f2d2807f5db74747f4dfe8f1fe89f9388f717347bbea9e9ec2931bb6fdc4b11648dfa9e726cdf690d74970a36f7482c12593a5ad7b966c4cf14655e11641f0bb67b8e807377edfa81055480da52031e0ba86ec52f991eb3cb8913c8f807287f3cb5ac4143326f33a4503cf31e021c8f41a5eec01870e0004acc0d0bf2bed65da5eeae3703ae878c20bd7f1167745e96770979146463fa40235e6bba8bdac1273dcbc1256cda0caacbdaad\">\n\r\n\t\t\r\n\t\t<div class=\"form-item type-id\">\r\n\t\t\t<label class=\"item-label\" for=\"sqexid\"><span class=\"label-image-text\" title=\"Square Enix ID\"></span></label>\r\n\t\t\t<input class=\"item-input\" name=\"sqexid\" id=\"sqexid\" type=\"text\" value=\"\" tabindex=\"1\" placeholder=\"ID (Required)\"  maxLength=\"16\"\r\n\t\t\t\r\n\t\t\t\t />\r\n\t\t\t\r\n\t\t</div>\r\n\r\n\t\t <div class=\"form-item type-pw\">\r\n\t\t\t<label class=\"item-label\" for=\"password\"><span class=\"label-image-text\" title=\"Square Enix Password\"></span></label>\r\n\t\t\t<input class=\"item-password\" name=\"password\" id=\"password\" type=\"password\" value=\"\" tabindex=\"2\" placeholder=\"Password (Required)\" maxLength=\"32\" autocomplete=\"off\"/>\r\n\t\t</div>\r\n\t\r\n\t\t<div class=\"form-item type-otpw\">\r\n\t\t\t<label class=\"item-label\" for=\"otppw\"><span class=\"label-image-text\" title=\"One-Time Password\"></span></label>\r\n\t\t\t<input class=\"item-otpw\" name=\"otppw\" id=\"otppw\" type=\"text\" value=\"\" tabindex=\"3\" autocomplete=\"off\" maxLength=\"6\" placeholder=\"Password (Optional)\" />\r\n\t\t</div>\r\n\r\n\t\t\r\n\t\t<div class=\"form-item type-remember-id\">\r\n\t\t\t<input name=\"saveid\" id=\"saveid\" type=\"checkbox\" value=\"1\" class=\"item-checkbox\" tabindex=\"4\"  />\r\n\t\t\t<label class=\"item-checkbox-label\" for=\"saveid\"><span class=\"label-checkbox-image-text\" title=\"Remember Square Enix ID\"></span></label>\r\n\t\t</div>\r\n\t\t\r\n\r\n\t\t<div class=\"form-item type-submit\">\r\n\t\t\t<button class=\"item-button\" type=\"submit\" tabindex=\"5\" onClick=\"ctrEvent('mainForm')\" id=\"btLogin\"><span class=\"button-image-text\" title=\"Login\"></span></button>\r\n\t\t</div>\r\n\r\n\t</form>\r\n</div>\r\n</body>\r\n</html>\r\n\r\n</html>",
    )
}

#[derive(Deserialize, Debug)]
#[allow(dead_code, non_snake_case)]
struct Input {
    _STORED_: String,
    sqexid: String,
    password: String,
    otppw: String,
}

#[derive(Serialize, Debug)]
#[allow(dead_code, non_snake_case)]
struct SapphireLogin {
    username: String,
    pass: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code, non_snake_case)]
struct SapphireLoginResponse {
    #[serde(rename = "frontierHost")]
    frontier_host: String,
    #[serde(rename = "lobbyHost")]
    lobby_Host: String,
    #[serde(rename = "lobbyPort")]
    lobby_port: i32,
    #[serde(rename = "sId")]
    sid: String,
}

async fn login_send(
    State(state): State<LoginServerState>,
    Form(input): Form<Input>,
) -> Html<String> {
    let config = get_config();
    if config.enable_sapphire_proxy {
        let sapphire_login = SapphireLogin {
            username: input.sqexid,
            pass: input.password,
        };
        let body = serde_json::to_string(&sapphire_login).unwrap();

        let Ok(mut login_reply) = ureq::post(format!(
            "http://{}/sapphire-api/lobby/login",
            config.sapphire_api_server
        ))
        .send(body) else {
            tracing::warn!("Failed to contact Sapphire API, is it running?");
            return Html(
                "window.external.user(\"login=auth,ng,err,Failed to contact Sapphire API\");"
                    .to_string(),
            );
        };

        let Ok(body) = login_reply.body_mut().read_to_string() else {
            return Html(
                "window.external.user(\"login=auth,ng,err,Failed to contact Sapphire API\");"
                    .to_string(),
            );
        };

        if body.is_empty() {
            return Html("window.external.user(\"login=auth,ng,err,Login failed\");".to_string());
        }

        let response: SapphireLoginResponse = serde_json::from_str(&body).unwrap();

        Html(format!(
            "window.external.user(\"login=auth,ok,sid,{},terms,1,region,2,etmadd,0,playable,1,ps3pkg,0,maxex,5,product,1\");",
            response.sid,
        ))
    } else {
        let user = state
            .database
            .login_user(GAME_SERVICE, &input.sqexid, &input.password);
        match user {
            Ok(session_id) => {
                let user_id = state.database.get_user_id(&session_id).unwrap();
                let max_ex = state.database.get_user_max_expansion(user_id).unwrap();

                Html(format!(
                    "window.external.user(\"login=auth,ok,sid,{session_id},terms,1,region,2,etmadd,0,playable,1,ps3pkg,0,maxex,{max_ex},product,1\");"
                ))
            }
            Err(err) => {
                // TODO: see what the official error messages are
                match err {
                    LoginError::WrongUsername => Html(
                        "window.external.user(\"login=auth,ng,err,Wrong Username\");".to_string(),
                    ),
                    LoginError::WrongPassword => Html(
                        "window.external.user(\"login=auth,ng,err,Wrong Password\");".to_string(),
                    ),
                    LoginError::InternalError => Html(
                        "window.external.user(\"login=auth,ng,err,Internal Server Error\");"
                            .to_string(),
                    ),
                }
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct RegisterInput {
    username: Option<String>,
    password: Option<String>,
}

async fn do_register(
    jar: CookieJar,
    State(state): State<LoginServerState>,
    Form(input): Form<RegisterInput>,
) -> (CookieJar, Redirect) {
    let config = get_config();

    // Redirect if registration is disabled, and they tried to be smart.
    if !config.login.enable_registration {
        return (
            CookieJar::default(),
            Redirect::to(&format!("{}/", config.web.server_name)),
        );
    }

    tracing::info!(
        "Registering with {:#?} and {:#?}!",
        input.username,
        input.password
    );

    let Some(username) = input.username else {
        panic!("Expected username!");
    };
    let Some(password) = input.password else {
        panic!("Expected password!");
    };

    if config.enable_sapphire_proxy {
        let sapphire_login = SapphireLogin {
            username,
            pass: password,
        };
        let body = serde_json::to_string(&sapphire_login).unwrap();

        let _ = ureq::post(format!(
            "http://{}/sapphire-api/lobby/createAccount",
            config.sapphire_api_server
        ))
        .send(body);

        // TODO: don't redirect to account management page, we can't do that for sapphire
        (jar, Redirect::to("/account/app/svc/manage"))
    } else {
        state.database.add_user(&username, &password);

        // redirect to account management page
        let sid = state
            .database
            .login_user(ACCOUNT_MANAGEMENT_SERVICE, &username, &password)
            .unwrap();

        let cookie = Cookie::build(("cis_sessid", sid))
            .path("/")
            .secure(false)
            .expires(Expiration::Session)
            .http_only(true);
        (jar.add(cookie), Redirect::to("/account/app/svc/manage"))
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CheckSessionParams {
    sid: String,
    service: String,
}

async fn check_session(
    State(state): State<LoginServerState>,
    Query(params): Query<CheckSessionParams>,
) -> String {
    let accounts = state.database.check_session(&params.service, &params.sid);
    serde_json::to_string(&accounts).unwrap_or(String::new())
}

async fn get_users(State(state): State<LoginServerState>) -> String {
    let users = state.database.get_users();
    serde_json::to_string(&users).unwrap_or(String::new())
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct MaxExParams {
    service: String,
}

async fn get_max_ex(
    State(state): State<LoginServerState>,
    Query(params): Query<MaxExParams>,
) -> String {
    // TODO: introduce a better failure state
    let max_ex = state
        .database
        .get_max_expansion(params.service.parse().unwrap());
    max_ex.unwrap_or(0).to_string()
}

async fn login() -> Html<String> {
    let config = get_config();
    let environment = setup_default_environment();
    let template = environment.get_template("login.html").unwrap();
    Html(
        template
            .render(context! { web_server_name => config.web.server_name, enable_registration => config.login.enable_registration })
            .unwrap(),
    )
}

async fn register() -> Response<Body> {
    let config = get_config();

    // Redirect if registration is disabled, and they tried to be smart.
    if !config.login.enable_registration {
        return Redirect::to(&format!("{}/", config.web.server_name)).into_response();
    }

    let environment = setup_default_environment();
    let template = environment.get_template("register.html").unwrap();
    Html(
        template
            .render(context! { web_server_name => config.web.server_name, enable_registration => config.login.enable_registration })
            .unwrap(),
    ).into_response()
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct LoginInput {
    username: Option<String>,
    password: Option<String>,
}

async fn do_login(
    State(state): State<LoginServerState>,
    jar: CookieJar,
    Form(input): Form<LoginInput>,
) -> (CookieJar, Redirect) {
    tracing::info!("{:#?} logging in!", input.username,);

    let Some(username) = input.username else {
        panic!("Expected username!");
    };
    let Some(password) = input.password else {
        panic!("Expected password!");
    };

    let sid = state
        .database
        .login_user(ACCOUNT_MANAGEMENT_SERVICE, &username, &password)
        .unwrap();

    let cookie = Cookie::build(("cis_sessid", sid))
        .path("/")
        .secure(false)
        .expires(Expiration::Session)
        .http_only(true);

    (jar.add(cookie), Redirect::to("/account/app/svc/manage"))
}

async fn account(State(state): State<LoginServerState>, jar: CookieJar) -> Html<String> {
    if let Some(session_id) = jar.get("cis_sessid")
        && let Some(user_id) = state.database.get_user_id(session_id.value())
    {
        let username = state.database.get_username(user_id);

        let environment = setup_default_environment();
        let template = environment.get_template("account.html").unwrap();
        return Html(template.render(context! { username => username }).unwrap());
    }

    Html("You need to be logged in!".to_string())
}

async fn upload_character_backup(
    State(state): State<LoginServerState>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Response<Body> {
    if let Some(session_id) = jar.get("cis_sessid")
        && let Some(user_id) = state.database.get_user_id(session_id.value())
    {
        let service_account_id = state.database.get_service_account(user_id);

        while let Some(field) = multipart.next_field().await.unwrap() {
            let name = field.name().unwrap().to_string();
            let data = field.bytes().await.unwrap();

            std::fs::write("temp.zip", data).unwrap();

            if name == "charbak" {
                let ipc_segment = CustomIpcSegment::new(CustomIpcData::ImportCharacter {
                    service_account_id,
                    path: "temp.zip".to_string(),
                });

                if let Some(response) = send_custom_world_packet(ipc_segment).await
                    && let CustomIpcData::CharacterImported { message } = response.data
                {
                    return restore_backup_with_message(message).await.into_response();
                }
            }
        }
    }

    restore_backup_with_message("Unknown Error".to_string())
        .await
        .into_response()
}

async fn logout(State(state): State<LoginServerState>, jar: CookieJar) -> (CookieJar, Redirect) {
    let config = get_config();
    if let Some(session_id) = jar.get("cis_sessid")
        && let Some(user_id) = state.database.get_user_id(session_id.value())
    {
        state
            .database
            .revoke_session(user_id, ACCOUNT_MANAGEMENT_SERVICE);
    }

    (
        jar.remove("cis_sessid"),
        Redirect::to(&format!("{}/", config.web.server_name)),
    )
}

async fn change_password() -> Html<String> {
    // TODO: actually change password
    let environment = setup_default_environment();
    let template = environment.get_template("changepassword.html").unwrap();
    Html(template.render(context! {}).unwrap())
}

async fn cancel_account() -> Html<String> {
    let environment = setup_default_environment();
    let template = environment.get_template("cancel.html").unwrap();
    Html(template.render(context! {}).unwrap())
}

async fn cancel_account_perform(
    State(state): State<LoginServerState>,
    jar: CookieJar,
) -> (CookieJar, Redirect) {
    if let Some(session_id) = jar.get("cis_sessid")
        && let Some(user_id) = state.database.get_user_id(session_id.value())
    {
        // TODO: only supports one service account
        let service_account_id = state.database.get_service_account(user_id);

        state.database.delete_user(user_id);

        let ipc_segment =
            CustomIpcSegment::new(CustomIpcData::DeleteServiceAccount { service_account_id });

        let _ = send_custom_world_packet(ipc_segment).await; // we don't care about the response, for now.
    }

    (jar.remove("cis_sessid"), Redirect::to("/"))
}

async fn restore_backup() -> Html<String> {
    let environment = setup_default_environment();
    let template = environment.get_template("restore.html").unwrap();
    Html(template.render(context! {}).unwrap())
}

async fn restore_backup_with_message(status_message: String) -> Html<String> {
    let environment = setup_default_environment();
    let template = environment.get_template("restore.html").unwrap();
    Html(
        template
            .render(context! { status_message => status_message })
            .unwrap(),
    )
}

async fn login_history(State(state): State<LoginServerState>, jar: CookieJar) -> Html<String> {
    if let Some(session_id) = jar.get("cis_sessid")
        && let Some(user_id) = state.database.get_user_id(session_id.value())
    {
        let environment = setup_default_environment();
        let template = environment.get_template("loginhistory.html").unwrap();
        let past_logins = state.database.get_sessions(user_id);

        return Html(
            template
                .render(context! { past_logins => past_logins, game_service_name => GAME_SERVICE })
                .unwrap(),
        );
    }

    Html("You need to be logged in!".to_string())
}

async fn login_history_with_sid(
    State(state): State<LoginServerState>,
    jar: CookieJar,
    generated_sid: &str,
) -> Html<String> {
    // TODO: de-duplicate with above function pls
    if let Some(session_id) = jar.get("cis_sessid")
        && let Some(user_id) = state.database.get_user_id(session_id.value())
    {
        let environment = setup_default_environment();
        let template = environment.get_template("loginhistory.html").unwrap();
        let past_logins = state.database.get_sessions(user_id);

        return Html(
                template
                    .render(context! { past_logins => past_logins, generated_sid => generated_sid, game_service_name => GAME_SERVICE })
                    .unwrap(),
            );
    }

    Html("You need to be logged in!".to_string())
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct SIDInput {
    service: Option<String>,
}

async fn manual_generate_sid(
    State(state): State<LoginServerState>,
    jar: CookieJar,
    Form(input): Form<SIDInput>,
) -> Response<Body> {
    let Some(service) = input.service else {
        panic!("Expected service!");
    };

    if let Some(session_id) = jar.get("cis_sessid")
        && state
            .database
            .is_session_valid(ACCOUNT_MANAGEMENT_SERVICE, session_id.value())
        && let Some(user_id) = state.database.get_user_id(session_id.value())
    {
        let new_sid = state
            .database
            .create_session(&service, user_id)
            .expect("Failed to create new SID?!");

        return login_history_with_sid(State(state), jar, &new_sid)
            .await
            .into_response();
    }

    login_history(State(state), jar).await.into_response()
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct RevokeInput {
    service: Option<String>,
}

async fn revoke_sid(
    State(state): State<LoginServerState>,
    jar: CookieJar,
    Form(input): Form<RevokeInput>,
) -> Response<Body> {
    let Some(service) = input.service else {
        panic!("Expected service!");
    };

    if let Some(session_id) = jar.get("cis_sessid")
        && state
            .database
            .is_session_valid(ACCOUNT_MANAGEMENT_SERVICE, session_id.value())
        && let Some(user_id) = state.database.get_user_id(session_id.value())
    {
        state.database.revoke_session(user_id, &service);
    }

    login_history(State(state), jar).await.into_response()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = LoginServerState {
        database: Arc::new(LoginDatabase::new()),
    };

    let cors = CorsLayer::new().allow_origin(Any);

    let app = Router::new()
        // retail API
        .route("/oauth/ffxivarr/login/top", get(top))
        .route("/oauth/ffxivarr/login/login.send", post(login_send))
        // private server<->server API
        // TODO: make these actually private
        .route("/_private/service_accounts", get(check_session))
        .route("/_private/users", get(get_users))
        .route("/_private/max_ex", get(get_max_ex))
        // public website
        .route("/oauth/oa/oauthlogin", get(login))
        .route("/oauth/oa/oauthlogin", post(do_login))
        .route("/oauth/oa/registligt", get(register))
        .route("/oauth/oa/registlist", post(do_register))
        .route("/account/app/svc/manage", get(account))
        .route("/account/app/svc/logout", get(logout))
        .route("/account/app/svc/mbrPasswd", get(change_password))
        .route("/account/app/svc/mbrCancel", get(cancel_account))
        .route(
            "/account/app/svc/mbrCancel/perform",
            get(cancel_account_perform),
        )
        .route("/account/app/svc/restore", get(restore_backup))
        .route("/account/app/svc/restore", post(upload_character_backup))
        .route("/account/app/svc/loginhistory", get(login_history))
        .route("/account/app/svc/login_generate", post(manual_generate_sid))
        .route("/account/app/svc/login_revoke", post(revoke_sid))
        .with_state(state)
        .nest_service("/static", ServeDir::new(web_static_dir!("")))
        .layer(cors);

    let config = get_config();

    let addr = config.login.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
