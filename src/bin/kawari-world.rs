use kawari::packet::{SegmentType, State, parse_packet, send_keep_alive};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:7100").await.unwrap();

    tracing::info!("World server started on 7100");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let (mut read, mut write) = tokio::io::split(socket);

        let mut state = State {
            client_key: None,
            session_id: None,
        };

        tokio::spawn(async move {
            let mut buf = [0; 2056];
            loop {
                let n = read.read(&mut buf).await.expect("Failed to read data!");

                if n != 0 {
                    let segments = parse_packet(&buf[..n], &mut state).await;
                    for segment in &segments {
                        match &segment.segment_type {
                            SegmentType::Ipc { data } => {
                                panic!("The server is recieving a IPC response or unknown packet!")
                            }
                            SegmentType::KeepAlive { id, timestamp } => {
                                send_keep_alive(&mut write, &state, *id, *timestamp).await
                            }
                            _ => {
                                panic!("The server is recieving a response or unknown packet!")
                            }
                        }
                    }
                }
            }
        });
    }
}
