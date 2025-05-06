use std::sync::Arc;

use axum::extract::{Multipart, Query, State};
use axum::response::{Html, Redirect};
use axum::routing::post;
use axum::{Form, Router, routing::get};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, Expiration};
use kawari::config::get_config;
use kawari::ipc::kawari::{CustomIpcData, CustomIpcSegment, CustomIpcType};
use kawari::lobby::send_custom_world_packet;
use kawari::login::{LoginDatabase, LoginError};
use minijinja::{Environment, context};
use serde::Deserialize;
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
        "account_base.html",
        std::fs::read_to_string("resources/templates/account_base.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "login_base.html",
        std::fs::read_to_string("resources/templates/login_base.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "login.html",
        std::fs::read_to_string("resources/templates/login.html")
            .expect("Failed to find template!"),
    )
    .unwrap();
    env.add_template_owned(
        "register.html",
        std::fs::read_to_string("resources/templates/register.html")
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
        "changepassword.html",
        std::fs::read_to_string("resources/templates/changepassword.html")
            .expect("Failed to find template!"),
    )
    .unwrap();

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

async fn login_send(
    State(state): State<LoginServerState>,
    Form(input): Form<Input>,
) -> Html<String> {
    let user = state.database.login_user(&input.sqexid, &input.password);
    match user {
        Ok(session_id) => Html(format!(
            "window.external.user(\"login=auth,ok,sid,{session_id},terms,1,region,2,etmadd,0,playable,1,ps3pkg,0,maxex,5,product,1\");"
        )),
        Err(err) => {
            // TODO: see what the official error messages are
            match err {
                LoginError::WrongUsername => {
                    Html("window.external.user(\"login=auth,ng,err,Wrong Username\");".to_string())
                }
                LoginError::WrongPassword => {
                    Html("window.external.user(\"login=auth,ng,err,Wrong Password\");".to_string())
                }
                LoginError::InternalError => Html(
                    "window.external.user(\"login=auth,ng,err,Internal Server Error\");"
                        .to_string(),
                ),
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

    state.database.add_user(&username, &password);

    // redirect to account management page
    let sid = state.database.login_user(&username, &password).unwrap();

    let cookie = Cookie::build(("cis_sessid", sid))
        .path("/")
        .secure(false)
        .expires(Expiration::Session)
        .http_only(true);
    (jar.add(cookie), Redirect::to("/account/app/svc/manage"))
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CheckSessionParams {
    sid: String,
}

async fn check_session(
    State(state): State<LoginServerState>,
    Query(params): Query<CheckSessionParams>,
) -> String {
    let accounts = state.database.check_session(&params.sid);
    serde_json::to_string(&accounts).unwrap_or(String::new())
}

async fn login() -> Html<String> {
    let config = get_config();
    let environment = setup_default_environment();
    let template = environment.get_template("login.html").unwrap();
    Html(
        template
            .render(context! { web_server_name => config.web.server_name })
            .unwrap(),
    )
}

async fn register() -> Html<String> {
    let config = get_config();
    let environment = setup_default_environment();
    let template = environment.get_template("register.html").unwrap();
    Html(
        template
            .render(context! { web_server_name => config.web.server_name })
            .unwrap(),
    )
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

    let sid = state.database.login_user(&username, &password).unwrap();

    let cookie = Cookie::build(("cis_sessid", sid))
        .path("/")
        .secure(false)
        .expires(Expiration::Session)
        .http_only(true);

    (jar.add(cookie), Redirect::to("/account/app/svc/manage"))
}

async fn account(State(state): State<LoginServerState>, jar: CookieJar) -> Html<String> {
    if let Some(session_id) = jar.get("cis_sessid") {
        let user_id = state.database.get_user_id(session_id.value());
        let username = state.database.get_username(user_id);

        let environment = setup_default_environment();
        let template = environment.get_template("account.html").unwrap();
        Html(template.render(context! { username => username }).unwrap())
    } else {
        Html("You need to be logged in!".to_string())
    }
}

async fn upload_character_backup(
    State(state): State<LoginServerState>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Redirect {
    if let Some(session_id) = jar.get("cis_sessid") {
        let user_id = state.database.get_user_id(session_id.value());
        let service_account_id = state.database.get_service_account(user_id);

        while let Some(field) = multipart.next_field().await.unwrap() {
            let name = field.name().unwrap().to_string();
            let data = field.bytes().await.unwrap();

            std::fs::write("temp.zip", data).unwrap();

            if name == "charbak" {
                let ipc_segment = CustomIpcSegment {
                    unk1: 0,
                    unk2: 0,
                    op_code: CustomIpcType::ImportCharacter,
                    option: 0,
                    timestamp: 0,
                    data: CustomIpcData::ImportCharacter {
                        service_account_id,
                        path: "temp.zip".to_string(),
                    },
                };

                send_custom_world_packet(ipc_segment).await.unwrap();
            }
        }
    }

    Redirect::to("/account/app/svc/manage")
}

async fn logout(jar: CookieJar) -> (CookieJar, Redirect) {
    let config = get_config();
    // TODO: remove session from database
    (
        jar.remove("cis_sessid"),
        Redirect::to(&format!("http://{}/", config.web.server_name)),
    )
}

async fn change_password() -> Html<String> {
    // TODO: actually change password
    let environment = setup_default_environment();
    let template = environment.get_template("changepassword.html").unwrap();
    Html(template.render(context! {}).unwrap())
}

async fn cancel_account(jar: CookieJar) -> (CookieJar, Redirect) {
    // TODO: actually delete account
    (jar.remove("cis_sessid"), Redirect::to("/"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = LoginServerState {
        database: Arc::new(LoginDatabase::new()),
    };

    let app = Router::new()
        // retail API
        .route("/oauth/ffxivarr/login/top", get(top))
        .route("/oauth/ffxivarr/login/login.send", post(login_send))
        // private server<->server API
        // TODO: make these actually private
        .route("/_private/service_accounts", get(check_session))
        // public website
        .route("/oauth/oa/oauthlogin", get(login))
        .route("/oauth/oa/oauthlogin", post(do_login))
        .route("/oauth/oa/registligt", get(register))
        .route("/oauth/oa/registlist", post(do_register))
        .route("/account/app/svc/manage", get(account))
        .route("/account/app/svc/manage", post(upload_character_backup))
        .route("/account/app/svc/logout", get(logout))
        .route("/account/app/svc/mbrPasswd", get(change_password))
        .route("/account/app/svc/mbrCancel", get(cancel_account))
        .with_state(state)
        .nest_service("/static", ServeDir::new("resources/static"));

    let config = get_config();

    let addr = config.login.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
