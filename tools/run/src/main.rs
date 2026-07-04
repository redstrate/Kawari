use std::process::Stdio;
use tokio::process::Command;

async fn start_server(name: &str) {
    let mut dir = std::env::current_exe().expect("Couldn't get current executable path");
    dir.pop();

    let mut extension = std::env::consts::EXE_EXTENSION.to_string();
    if !extension.is_empty() {
        extension = format!(".{extension}");
    }

    dir.push(format!("{name}{}", extension));

    let library_path = if std::env::var("CARGO").is_ok() {
        "./oodle"
    } else {
        "."
    };

    Command::new(dir)
        .env("LD_LIBRARY_PATH", library_path) // ensure we find the oodle .so at the right location
        .env("RUST_BACKTRACE", "1") // Print backtraces on asserts
        .stdout(Stdio::inherit())
        .spawn()
        .expect("Failed to run server")
        .wait()
        .await
        .expect("Failed to run server");
}

#[tokio::main]
async fn main() {
    // Enables ANSI code support on Windows. See https://github.com/tokio-rs/tracing/issues/3068
    #[cfg(windows)]
    nu_ansi_term::enable_ansi_support().ok();

    // If being invoked by Cargo, build the workspace first.
    if let Ok(cargo) = std::env::var("CARGO") {
        let build_exit_status = Command::new(cargo)
            .args(if cfg!(debug_assertions) {
                vec!["build", "--features", "oodle"]
            } else {
                vec!["build", "--release", "--features", "oodle"]
            })
            .stdout(Stdio::inherit())
            .spawn()
            .expect("Failed to run Cargo build")
            .wait()
            .await
            .expect("Failed to run Cargo build");

        // Silently exit if build failed
        if !build_exit_status.success() {
            return;
        }
    }

    tokio::join!(
        start_server("kawari-admin"),
        start_server("kawari-frontier"),
        start_server("kawari-launcher"),
        start_server("kawari-lobby"),
        start_server("kawari-login"),
        start_server("kawari-patch"),
        start_server("kawari-web"),
        // kawari-world is intentionally NOT started here. The world server is run separately
        // (e.g. `cargo run -p kawari-world`) so it can be restarted on its own during development
        // without bouncing the other services.
    );
}
