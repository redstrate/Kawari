use minijinja::Environment;
use rand::distributions::Alphanumeric;
use rand::Rng;

pub mod config;

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
    env.add_template("admin.html", include_str!("../templates/admin.html")).unwrap();

    env
}