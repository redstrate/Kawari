use std::process::Stdio;

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

async fn start_servers(name: &str) -> Option<Child> {
    let mut cmd = Command::new(&*format!("./target/debug/{name}"));
    cmd.stdout(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn command");

    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let mut reader = BufReader::new(stdout).lines();

    loop {
        let stdout = reader.next_line().await.unwrap().unwrap();

        println!("out: {stdout}");

        if stdout.contains("Server started") {
            break;
        }
    }

    Some(child)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Kawari...");

    let server_executables = [
        "kawari-admin",
        "kawari-datacentertravel",
        "kawari-frontier",
        "kawari-launcher",
        "kawari-lobby",
        "kawari-login",
        "kawari-patch",
        "kawari-savedatabank",
        "kawari-web",
        "kawari-world",
    ];

    for executable in server_executables {
        tokio::task::spawn(async {
            let child = start_servers(executable).await;
        })
        .await
        .unwrap();
    }

    println!("Kawari finished setting up!");

    let (tx, rx) = std::sync::mpsc::channel();

    ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
        .expect("Error setting Ctrl-C handler");

    println!("Waiting for Ctrl-C...");
    rx.recv().expect("Could not receive from channel.");

    println!("Got it! Exiting...");
}
