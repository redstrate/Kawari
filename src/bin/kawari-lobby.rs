use kawari::packet::parse_packet;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:7000").await.unwrap();

    tracing::info!("Lobby server started on 7000");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let (mut read, _) = tokio::io::split(socket);

        tokio::spawn(async move {
            let mut buf = [0; 2056];
            loop {
                let n = read
                .read(&mut buf)
                .await
                .expect("Failed to read data!");

                parse_packet(&buf[..n]);
            }
        });
    }
}
