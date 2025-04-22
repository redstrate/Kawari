use kawari::RECEIVE_BUFFER_SIZE;
use kawari::config::get_config;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = get_config();

    let addr = config.save_data_bank.get_socketaddr();

    let listener = TcpListener::bind(addr).await.unwrap();

    tracing::info!("Server started on {addr}");

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            loop {
                let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
                let n = socket.read(&mut buf).await.expect("Failed to read data!");

                if n != 0 {
                    dbg!(buf);
                }
            }
        });
    }
}
