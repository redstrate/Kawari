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
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/xml"),
            )],
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

async fn session_get_init(_body: String) -> Xml<Vec<u8>> {
    // TODO: just a guess
    Xml(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><result>
<return_code>OK</return_code>
<information/>
<inquiry_categoryList/>
<inquiry_itemList/>
<report_itemList/>
</result>"
            .as_bytes()
            .to_vec(),
    )
}

async fn view_get_init() -> Xml<Vec<u8>> {
    Xml(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><result>
<return_code>OK</return_code>
<information/>
<inquiry_categoryList/>
<inquiry_itemList/>
<report_itemList/>
</result>"
            .as_bytes()
            .to_vec(),
    )
}

#[derive(Serialize)]
#[serde(rename = "item")]
struct Item {
    title: String,
    published: i64,
    updated: i64,
    lsb_id: String,
    lsb_parentid: Option<String>,
    lsb_tag: Option<String>,
    #[serde(rename = "catId")]
    cat_id: i32,
    content: String,
}

#[derive(Serialize)]
#[serde(rename = "information")]
struct Information {
    #[serde(rename = "#content")]
    items: Vec<Item>,
}

#[derive(Serialize)]
#[serde(rename = "result")]
struct Result {
    return_code: String,
    information: Information,
}

async fn get_headline_all() -> Xml<Vec<u8>> {
    let result = Result {
        return_code: "OK".to_string(),
        information: Information {
            items: vec![Item {
                title: "Test".to_string(),
                published: 1752130800,
                updated: 1752130800,
                lsb_id: "c8819ec6f93f6c56d760b42c2ba2f43fe6598fc8".to_string(),
                lsb_parentid: None,
                lsb_tag: None,
                cat_id: 1,
                content: "Hello, world!".to_string(),
            }],
        },
    };

    Xml(serde_xml_rs::to_string(&result)
        .unwrap()
        .as_bytes()
        .to_vec())
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
