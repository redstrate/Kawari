use minijinja::Environment;
use rand::Rng;
use rand::distributions::Alphanumeric;

pub mod config;
pub mod encryption;
pub mod packet;
pub mod patchlist;
mod common;

pub fn generate_sid() -> String {
    let random_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(56)
        .map(char::from)
        .collect();
    random_id.to_lowercase()
}

pub fn setup_default_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("admin.html", include_str!("../templates/admin.html"))
        .unwrap();
    env.add_template("web.html", include_str!("../templates/web.html"))
        .unwrap();
    env.add_template("login.html", include_str!("../templates/login.html"))
        .unwrap();
    env.add_template("register.html", include_str!("../templates/register.html"))
        .unwrap();
    env.add_template(
        "worldstatus.html",
        include_str!("../templates/worldstatus.html"),
    )
    .unwrap();

    env
}
