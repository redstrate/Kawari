use binrw::binrw;

mod character_action;
pub use character_action::LobbyCharacterAction;

mod character_list;
pub use character_list::CharacterDetails;

mod client_version_info;

mod server_list;
pub use server_list::Server;

mod service_account_list;
pub use service_account_list::ServiceAccount;

use crate::{
    CHAR_NAME_MAX_LENGTH,
    common::{read_string, write_string},
    packet::{IpcSegment, IpcSegmentTrait},
};

pub type ClientLobbyIpcSegment = IpcSegment<ClientLobbyIpcType, ClientLobbyIpcData>;

impl IpcSegmentTrait for ClientLobbyIpcSegment {
    fn calc_size(&self) -> u32 {
        todo!()
    }
}

// TODO: make generic
impl Default for ClientLobbyIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ClientLobbyIpcType::ClientVersionInfo,
            server_id: 0,
            timestamp: 0,
            data: ClientLobbyIpcData::ClientVersionInfo {
                sequence: 0,
                session_id: String::new(),
                version_info: String::new(),
            },
        }
    }
}

pub type ServerLobbyIpcSegment = IpcSegment<ServerLobbyIpcType, ServerLobbyIpcData>;

impl IpcSegmentTrait for ServerLobbyIpcSegment {
    fn calc_size(&self) -> u32 {
        // 16 is the size of the IPC header
        16 + match self.op_code {
            ServerLobbyIpcType::LobbyError => 536,
            ServerLobbyIpcType::LobbyServiceAccountList => 24 + (8 * 80),
            ServerLobbyIpcType::LobbyCharacterList => 80 + (2 * 1184),
            ServerLobbyIpcType::LobbyEnterWorld => 160,
            ServerLobbyIpcType::LobbyServerList => 24 + (6 * 84),
            ServerLobbyIpcType::LobbyRetainerList => 210,
            ServerLobbyIpcType::CharacterCreated => 2568,
        }
    }
}

// TODO: make generic
impl Default for ServerLobbyIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ServerLobbyIpcType::LobbyError,
            server_id: 0,
            timestamp: 0,
            data: ServerLobbyIpcData::LobbyError {
                sequence: 0,
                error: 0,
                value: 0,
                exd_error_id: 0,
                unk1: 0,
            },
        }
    }
}

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, PartialEq, Debug)]
pub enum ServerLobbyIpcType {
    /// Sent by the server to indicate an lobby error occured
    LobbyError = 0x2,
    /// Sent by the server to inform the client of their service accounts.
    LobbyServiceAccountList = 0xC,
    /// Sent by the server to inform the client of their characters.
    LobbyCharacterList = 0xD,
    /// Sent by the server to tell the client how to connect to the world server.
    LobbyEnterWorld = 0xF,
    /// Sent by the server to inform the client of their servers.
    LobbyServerList = 0x15,
    /// Sent by the server to inform the client of their retainers.
    LobbyRetainerList = 0x17,
    // Assumed what this is, but probably incorrect
    CharacterCreated = 0xE,
}

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, PartialEq, Debug)]
pub enum ClientLobbyIpcType {
    /// Sent by the client when it requests the character list in the lobby.
    RequestCharacterList = 0x3,
    /// Sent by the client when it requests to enter a world.
    RequestEnterWorld = 0x4,
    /// Sent by the client after exchanging encryption information with the lobby server.
    ClientVersionInfo = 0x5,
    /// Sent by the client when they request something about the character (e.g. deletion.)
    LobbyCharacterAction = 0xB,
}

#[binrw]
#[br(import(magic: &ClientLobbyIpcType))]
#[derive(Debug, Clone)]
pub enum ClientLobbyIpcData {
    // Client->Server IPC
    #[br(pre_assert(*magic == ClientLobbyIpcType::ClientVersionInfo))]
    ClientVersionInfo {
        sequence: u64,

        #[brw(pad_before = 10)] // full of nonsense i don't understand yet
        #[br(count = 64)]
        #[br(map = read_string)]
        #[bw(ignore)]
        session_id: String,

        #[brw(pad_before = 8)] // empty
        #[br(count = 128)]
        #[br(map = read_string)]
        #[bw(ignore)]
        version_info: String,
        // unknown stuff at the end, it's not completely empty'
    },
    #[br(pre_assert(*magic == ClientLobbyIpcType::RequestCharacterList))]
    RequestCharacterList {
        #[brw(pad_before = 16)]
        sequence: u64,
        // TODO: what is in here?
    },
    #[br(pre_assert(*magic == ClientLobbyIpcType::LobbyCharacterAction))]
    LobbyCharacterAction {
        request_number: u32,
        unk1: u32,
        character_id: u64,
        #[br(pad_before = 8)]
        character_index: u8,
        action: LobbyCharacterAction,
        world_id: u16,
        #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        name: String,
        #[bw(pad_size_to = 436)]
        #[br(count = 436)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        json: String,
    },
    #[br(pre_assert(*magic == ClientLobbyIpcType::RequestEnterWorld))]
    RequestEnterWorld {
        #[brw(pad_before = 16)]
        sequence: u64,
        lookup_id: u64,
        // TODO: what else is in here?
    },
}

#[binrw]
#[br(import(_magic: &ServerLobbyIpcType))]
#[derive(Debug, Clone)]
pub enum ServerLobbyIpcData {
    LobbyServiceAccountList {
        #[br(dbg)]
        sequence: u64,
        #[brw(pad_before = 1)]
        num_service_accounts: u8,
        unk1: u8,
        #[brw(pad_after = 4)]
        unk2: u8,
        #[br(count = 8)]
        service_accounts: Vec<ServiceAccount>,
    },
    LobbyServerList {
        sequence: u64,
        unk1: u16,
        offset: u16,
        #[brw(pad_after = 8)]
        num_servers: u32,
        #[br(count = 6)]
        #[brw(pad_size_to = 504)]
        servers: Vec<Server>,
    },
    LobbyRetainerList {
        // TODO: what is in here?
        #[brw(pad_before = 7)]
        #[brw(pad_after = 202)]
        unk1: u8,
    },
    LobbyCharacterList {
        sequence: u64,
        counter: u8,
        #[brw(pad_after = 2)]
        num_in_packet: u8,
        unk1: u8,
        unk2: u8,
        unk3: u8,
        /// Set to 128 if legacy character
        unk4: u8,
        unk5: [u32; 7],
        unk6: u8,
        veteran_rank: u8,
        #[brw(pad_after = 1)]
        unk7: u8,
        days_subscribed: u32,
        remaining_days: u32,
        days_to_next_rank: u32,
        max_characters_on_world: u16,
        unk8: u16,
        #[brw(pad_after = 12)]
        entitled_expansion: u32,
        #[br(count = 2)]
        #[brw(pad_size_to = 2368)]
        characters: Vec<CharacterDetails>,
    },
    LobbyEnterWorld {
        sequence: u64,
        character_id: u32,
        #[brw(pad_before = 4)]
        content_id: u64,
        #[brw(pad_before = 4)]
        #[bw(pad_size_to = 66)]
        #[br(count = 66)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        session_id: String,
        port: u16,
        #[brw(pad_after = 16)]
        #[br(count = 48)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        host: String,
    },
    LobbyError {
        sequence: u64,
        error: u32,
        value: u32,
        exd_error_id: u16,
        unk1: u16,
    },
    NameRejection {
        // FIXME: This is opcode 0x2, which is InitializeChat. We need to separate the lobby/zone IPC codes.
        unk1: u8,
        #[brw(pad_before = 7)] // empty
        unk2: u16,
        #[brw(pad_before = 6)] // empty
        #[brw(pad_after = 516)] // mostly empty
        unk3: u32,
    },
    CharacterCreated {
        #[brw(pad_after = 4)] // empty
        unk1: u32,
        #[brw(pad_after = 4)] // empty
        unk2: u32,
        #[brw(pad_before = 32)] // empty
        #[brw(pad_after = 1136)] // empty
        details: CharacterDetails,
    },
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinWrite;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn lobby_ipc_sizes() {
        let ipc_types = [
            (
                ServerLobbyIpcType::LobbyServerList,
                ServerLobbyIpcData::LobbyServerList {
                    sequence: 0,
                    unk1: 0,
                    offset: 0,
                    num_servers: 0,
                    servers: Vec::new(),
                },
            ),
            (
                ServerLobbyIpcType::LobbyCharacterList,
                ServerLobbyIpcData::LobbyCharacterList {
                    sequence: 0,
                    counter: 0,
                    num_in_packet: 0,
                    unk1: 0,
                    unk2: 0,
                    unk3: 0,
                    unk4: 0,
                    unk5: [0; 7],
                    unk6: 0,
                    veteran_rank: 0,
                    unk7: 0,
                    days_subscribed: 0,
                    remaining_days: 0,
                    days_to_next_rank: 0,
                    max_characters_on_world: 0,
                    unk8: 0,
                    entitled_expansion: 0,
                    characters: Vec::new(),
                },
            ),
        ];

        for (opcode, ipc) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ServerLobbyIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: opcode.clone(),
                server_id: 0,
                timestamp: 0,
                data: ipc.clone(),
            };
            ipc_segment.write_le(&mut cursor).unwrap();

            let buffer = cursor.into_inner();

            assert_eq!(
                buffer.len(),
                ipc_segment.calc_size() as usize,
                "{:#?} did not match size!",
                opcode
            );
        }
    }
}
