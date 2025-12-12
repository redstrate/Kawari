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

    let library_path = if let Ok(_) = std::env::var("CARGO") {
        "./oodle"
    } else {
        "."
    };

    Command::new(dir)
        .env("LD_LIBRARY_PATH", library_path) // ensure we find the oodle .so at the right location
        .stdout(Stdio::inherit())
        .spawn()
        .expect("Failed to run server")
        .wait()
        .await
        .expect("Failed to run server");
}

#[tokio::main]
async fn main() {
    // If being invoked by Cargo, build the workspace first.
    if let Ok(cargo) = std::env::var("CARGO") {
        let build_exit_status = Command::new(cargo)
            .args(["build", "--features", "oodle"])
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
        start_server("kawari-datacentertravel"),
        start_server("kawari-frontier"),
        start_server("kawari-launcher"),
        start_server("kawari-lobby"),
        start_server("kawari-login"),
        start_server("kawari-patch"),
        start_server("kawari-savedatabank"),
        start_server("kawari-web"),
        start_server("kawari-world"),
    );

    println!("test");
}
