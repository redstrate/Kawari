mod chara_make;
pub use chara_make::CharaMake;

mod client_select_data;

mod connection;
pub use connection::LobbyConnection;

/// The IPC packets for the Lobby connection.
pub mod ipc;
