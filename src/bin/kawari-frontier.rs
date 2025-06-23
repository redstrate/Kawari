use axum::{
    Json, Router,
    http::{HeaderValue, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kawari::config::get_config;
use reqwest::{StatusCode, header};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn get_login_status() -> Json<GateStatus> {
    tracing::info!("Requesting login status...");

    let config = get_config();
    Json(GateStatus {
        status: config.frontier.login_open.into(),
    })
}

async fn get_world_status() -> Json<GateStatus> {
    tracing::info!("Requesting world status...");

    let config = get_config();
    Json(GateStatus {
        status: config.frontier.worlds_open.into(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Banner {
    link: String,
    lsb_banner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NewsItem {
    date: String,
    id: String,
    tag: String,
    title: String,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Headline {
    banner: Vec<Banner>,
    news: Vec<NewsItem>,
    pinned: Vec<NewsItem>,
    topics: Vec<NewsItem>,
}

async fn get_headline() -> Json<Headline> {
    tracing::info!("Requesting headline...");

    Json(Headline {
        banner: vec![],
        news: vec![NewsItem {
            date: "".to_string(),
            id: "".to_string(),
            tag: "".to_string(),
            title: "You are connected to Kawari".to_string(),
            url: "https://github.com/redstrate/Kawari".to_string(),
        }],
        pinned: vec![],
        topics: vec![],
    })
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    tracing::warn!("{}", uri);
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}

#[derive(Clone, Copy, Debug)]
#[must_use]
pub struct Xml<T>(pub T);

impl<T> IntoResponse for Xml<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        (
            [(header::CONTENT_TYPE, HeaderValue::from_static("text/xml"))],
            self.0,
        )
            .into_response()
    }
}

impl<T> From<T> for Xml<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

async fn session_get_init() -> Xml<String> {
    // TODO: just a guess
    Xml("<result>
<return_code>OK</return_code>
<information/>
<inquiry_categoryList/>
<inquiry_itemList/>
<report_itemList/>
</result>"
        .to_string())
}

async fn view_get_init() -> Xml<String> {
    Xml("<result>
<return_code>OK</return_code>
<information/>
<inquiry_categoryList/>
<inquiry_itemList/>
<report_itemList/>
</result>"
        .to_string())
}

async fn get_headline_all() -> Xml<String> {
    Xml("<result>
<return_code>OK</return_code>
<information>
</information>
</result>"
        .to_string())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/worldStatus/gate_status.json", get(get_world_status))
        .route("/worldStatus/login_status.json", get(get_login_status))
        .route("/news/headline.json", get(get_headline))
        // used by the client
        .route(
            "/frontier-api/ffxivsupport/session/get_init",
            post(session_get_init),
        )
        .route(
            "/frontier-api/ffxivsupport/view/get_init",
            get(view_get_init),
        )
        .route(
            "/frontier-api/ffxivsupport/information/get_headline_all",
            get(get_headline_all),
        )
        .fallback(fallback)
        .nest_service("/static", ServeDir::new("resources/static"));

    let config = get_config();

    let addr = config.frontier.get_socketaddr();
    tracing::info!("Server started on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
