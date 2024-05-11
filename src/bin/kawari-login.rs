use std::net::SocketAddr;

use axum::{Form, Router, routing::get};
use axum::extract::Query;
use axum::response::Html;
use axum::routing::post;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::Deserialize;

#[derive(Deserialize)]
#[allow(dead_code)]
struct Params {
    lng: String,
    rgn: String,
    isft: String,
    cssmode: String,
    isnew: String,
    launchver: String
}

async fn top(Query(params): Query<Params>) -> Html<&'static str> {
    Html("\r\n<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//EN\" \"http://www.w3.org/TR/html4/loose.dtd\">\r\n<html lang=en-GB id=gb>\r\n<head>\r\n<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\" /></head><form action=\"login.send\" method=\"post\" name=\"mainForm\">\r\n\t\r\n\t\t\r\n\t\t<input type=\"hidden\" name=\"_STORED_\" value=\"42f06e5f4194001a9ad61c8481f435e8b9eac79242f9221d463aa492ab2b3373655adadff3e72dd16a798ee8a222c519848743c97084f1af71854f06050a1f2813e5c3aaf66e5f0ef24dc18588a8cf06758992e42035f7e4f99f85c8b6082200dcabc6a37c7f76ce542eefc1f1798da5e23fd4b46ed17489de5eb8e8a222c5198487433bff5f3433c061ded661b3f33b5f2d2807f5db74747f4dfe8f1fe89f9388f717347bbea9e9ec2931bb6fdc4b11648dfa9e726cdf690d74970a36f7482c12593a5ad7b966c4cf14655e11641f0bb67b8e807377edfa81055480da52031e0ba86ec52f991eb3cb8913c8f807287f3cb5ac4143326f33a4503cf31e021c8f41a5eec01870e0004acc0d0bf2bed65da5eeae3703ae878c20bd7f1167745e96770979146463fa40235e6bba8bdac1273dcbc1256cda0caacbdaad\">\n\r\n\t\t\r\n\t\t<div class=\"form-item type-id\">\r\n\t\t\t<label class=\"item-label\" for=\"sqexid\"><span class=\"label-image-text\" title=\"Square Enix ID\"></span></label>\r\n\t\t\t<input class=\"item-input\" name=\"sqexid\" id=\"sqexid\" type=\"text\" value=\"\" tabindex=\"1\" placeholder=\"ID (Required)\"  maxLength=\"16\"\r\n\t\t\t\r\n\t\t\t\t />\r\n\t\t\t\r\n\t\t</div>\r\n\r\n\t\t <div class=\"form-item type-pw\">\r\n\t\t\t<label class=\"item-label\" for=\"password\"><span class=\"label-image-text\" title=\"Square Enix Password\"></span></label>\r\n\t\t\t<input class=\"item-password\" name=\"password\" id=\"password\" type=\"password\" value=\"\" tabindex=\"2\" placeholder=\"Password (Required)\" maxLength=\"32\" autocomplete=\"off\"/>\r\n\t\t</div>\r\n\t\r\n\t\t<div class=\"form-item type-otpw\">\r\n\t\t\t<label class=\"item-label\" for=\"otppw\"><span class=\"label-image-text\" title=\"One-Time Password\"></span></label>\r\n\t\t\t<input class=\"item-otpw\" name=\"otppw\" id=\"otppw\" type=\"text\" value=\"\" tabindex=\"3\" autocomplete=\"off\" maxLength=\"6\" placeholder=\"Password (Optional)\" />\r\n\t\t</div>\r\n\r\n\t\t\r\n\t\t<div class=\"form-item type-remember-id\">\r\n\t\t\t<input name=\"saveid\" id=\"saveid\" type=\"checkbox\" value=\"1\" class=\"item-checkbox\" tabindex=\"4\"  />\r\n\t\t\t<label class=\"item-checkbox-label\" for=\"saveid\"><span class=\"label-checkbox-image-text\" title=\"Remember Square Enix ID\"></span></label>\r\n\t\t</div>\r\n\t\t\r\n\r\n\t\t<div class=\"form-item type-submit\">\r\n\t\t\t<button class=\"item-button\" type=\"submit\" tabindex=\"5\" onClick=\"ctrEvent('mainForm')\" id=\"btLogin\"><span class=\"button-image-text\" title=\"Login\"></span></button>\r\n\t\t</div>\r\n\r\n\t</form>\r\n</div>\r\n</body>\r\n</html>\r\n\r\n</html>")
}

#[derive(Deserialize, Debug)]
#[allow(dead_code, non_snake_case)]
struct Input {
    _STORED_: String,
    sqexid: String,
    password: String,
    otppw: String
}

async fn login_send(Form(input): Form<Input>) -> Html<String>  {
    let random_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(56)
        .map(char::from)
        .collect();
    let sid = random_id.to_lowercase();

    Html(format!("window.external.user(\"login=auth,ok,sid,{sid},terms,1,region,2,etmadd,0,playable,1,ps3pkg,0,maxex,4,product,1\");"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/oauth/ffxivarr/login/top", get(top))
        .route("/oauth/ffxivarr/login/login.send", post(login_send));

    let addr = SocketAddr::from(([127, 0, 0, 1], 6700));
    tracing::info!("Login server started on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}