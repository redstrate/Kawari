use axum::{
    Json, Router,
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kawari::config::get_config;
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;
use xml::EmitterConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateStatus {
    status: i32,
}

async fn get_login_status() -> Json<GateStatus> {
    let config = get_config();
    Json(GateStatus {
        status: config.frontier.login_open.into(),
    })
}

async fn get_world_status() -> Json<GateStatus> {
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
    tracing::warn!("Unhandled route {}", uri);
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
    Xml(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><result><return_code>OK</return_code><sessionKey>aaaa</sessionKey></result>"#
        .as_bytes()
        .to_vec(),
    )
}

async fn view_get_init() -> Xml<Vec<u8>> {
    let result = ViewInitResult {
        return_code: "OK".to_string(),
        ..Default::default()
    };

    // FFXIV's XML parser cannot parse the padding
    let config = EmitterConfig::new().pad_self_closing(false);
    let xml = serde_xml_rs::SerdeXml::new().emitter(config);

    Xml(xml.to_string(&result).unwrap().as_bytes().to_vec())
}

#[derive(Serialize, Debug)]
#[serde(rename = "item")]
struct Item {
    /// Title of the item.
    title: String,
    /// UNIX timestamp of when this item was published.
    published: i64,
    /// UNIX timestamp of when this item was updated.
    updated: i64,
    /// Unique ID of this item.
    lsb_id: String,
    /// ID of the parent item, if applicable.
    lsb_parentid: Option<String>,
    /// Tag for this item.
    lsb_tag: Option<String>,
    /// Category for this item.
    #[serde(rename = "catId")]
    cat_id: i32,
    /// Text description for this item.
    content: String,
}

#[derive(Serialize, Default)]
struct Information {
    #[serde(rename = "#content")]
    items: Vec<Item>,
}

#[derive(Serialize, Default)]
struct SubCategory {
    label: String,
    #[serde(rename = "subCategoryId")]
    sub_category_id: i64,
}

#[derive(Serialize, Default)]
struct SubCategoryList {
    #[serde(rename = "subCategory")]
    items: Vec<SubCategory>,
}

#[derive(Serialize, Default)]
#[serde(rename = "mainCategory")]
struct MainCategory {
    label: String,
    #[serde(rename = "mainCategoryId")]
    main_category_id: i64,
    #[serde(rename = "subCategoryList")]
    subcategories: SubCategoryList,
}

#[derive(Serialize, Default)]
struct InquiryCategoryList {
    #[serde(rename = "#content")]
    categories: Vec<MainCategory>,
}

#[derive(Serialize, Default)]
#[serde(rename = "item")]
struct InquiryItemListItem {
    title: String,
    kid: i64,
}

#[derive(Serialize, Default)]
struct InquiryItemList {
    #[serde(rename = "#content")]
    items: Vec<InquiryItemListItem>,
}

#[derive(Serialize, Default)]
struct ReportItemList {}

#[derive(Serialize, Default)]
#[serde(rename = "result")]
struct ViewInitResult {
    #[serde(rename = "return_code")]
    return_code: String,
    #[serde(rename = "information")]
    information: Information,
    #[serde(rename = "inquiry_categoryList")]
    inquiry_category_list: InquiryCategoryList,
    #[serde(rename = "inquiry_itemList")]
    inquiry_item_list: InquiryItemList,
    #[serde(rename = "report_itemList")]
    report_item_list: ReportItemList,
}

#[derive(Serialize, Default)]
#[serde(rename = "result")]
struct HeadlineResult {
    #[serde(rename = "return_code")]
    return_code: String,
    #[serde(rename = "information")]
    information: Information,
}

async fn get_headline_all() -> Xml<Vec<u8>> {
    let result = HeadlineResult {
        return_code: "OK".to_string(),
        ..Default::default()
    };

    let config = EmitterConfig::new().pad_self_closing(false);
    let xml = serde_xml_rs::SerdeXml::new().emitter(config);

    Xml(xml.to_string(&result).unwrap().as_bytes().to_vec())
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
