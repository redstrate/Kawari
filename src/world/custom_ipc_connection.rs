use crate::{
    common::{
        GameData, determine_initial_starting_zone,
        workdefinitions::{CharaMake, RemakeMode},
    },
    config::get_config,
    inventory::Inventory,
    ipc::kawari::{CustomIpcData, CustomIpcSegment},
    packet::{
        CompressionType, ConnectionState, ConnectionType, PacketSegment, SegmentData, SegmentType,
        parse_packet, send_packet,
    },
    world::WorldDatabase,
};

use std::sync::{Arc, Mutex};

use std::net::SocketAddr;
use std::time::Instant;
use tokio::net::TcpStream;

/// Represents a single connection between an instance of the world server and the lobby server.
pub struct CustomIpcConnection {
    pub socket: TcpStream,
    pub ip: SocketAddr,
    pub state: ConnectionState,
    pub last_keep_alive: Instant,
    pub database: Arc<WorldDatabase>,
    pub gamedata: Arc<Mutex<GameData>>,
}

impl CustomIpcConnection {
    pub fn parse_packet(
        &mut self,
        data: &[u8],
    ) -> (Vec<PacketSegment<CustomIpcSegment>>, ConnectionType) {
        parse_packet(data, &mut self.state)
    }

    pub async fn send_custom_response(&mut self, segment: PacketSegment<CustomIpcSegment>) {
        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::KawariIpc,
            CompressionType::Uncompressed,
            &[segment],
        )
        .await;
    }

    pub async fn handle_custom_ipc(&mut self, data: &CustomIpcSegment) {
        match &data.data {
            CustomIpcData::RequestCreateCharacter {
                service_account_id,
                name,
                chara_make_json,
            } => {
                tracing::info!("creating character from: {name} {chara_make_json}");

                let chara_make = CharaMake::from_json(chara_make_json);

                let city_state;
                {
                    let mut game_data = self.gamedata.lock().unwrap();

                    city_state = game_data
                        .get_citystate(chara_make.classjob_id as u16)
                        .expect("Unknown citystate");
                }

                let mut inventory = Inventory::default();
                let (content_id, actor_id);
                {
                    let mut game_data = self.gamedata.lock().unwrap();

                    inventory.equip_classjob_items(chara_make.classjob_id as u16, &mut game_data);

                    // fill inventory
                    inventory.equip_racial_items(
                        chara_make.customize.race,
                        chara_make.customize.gender,
                        &mut game_data,
                    );

                    (content_id, actor_id) = self.database.create_player_data(
                        *service_account_id,
                        name,
                        chara_make_json,
                        city_state,
                        determine_initial_starting_zone(city_state),
                        inventory,
                        &mut game_data,
                    );
                }

                tracing::info!("Created new player: {content_id} {actor_id}");

                // send them the new actor and content id
                {
                    self.send_custom_response(PacketSegment {
                        segment_type: SegmentType::KawariIpc,
                        data: SegmentData::KawariIpc(CustomIpcSegment::new(
                            CustomIpcData::CharacterCreated {
                                actor_id,
                                content_id,
                            },
                        )),
                        ..Default::default()
                    })
                    .await;
                }
            }
            CustomIpcData::GetActorId { content_id } => {
                let actor_id = self.database.find_actor_id(*content_id);

                tracing::info!("We found an actor id: {actor_id}");

                // send them the actor id
                {
                    self.send_custom_response(PacketSegment {
                        segment_type: SegmentType::KawariIpc,
                        data: SegmentData::KawariIpc(CustomIpcSegment::new(
                            CustomIpcData::ActorIdFound { actor_id },
                        )),
                        ..Default::default()
                    })
                    .await;
                }
            }
            CustomIpcData::CheckNameIsAvailable { name } => {
                let is_name_free = self.database.check_is_name_free(name);

                // send response
                {
                    self.send_custom_response(PacketSegment {
                        segment_type: SegmentType::KawariIpc,
                        data: SegmentData::KawariIpc(CustomIpcSegment::new(
                            CustomIpcData::NameIsAvailableResponse { free: is_name_free },
                        )),
                        ..Default::default()
                    })
                    .await;
                }
            }
            CustomIpcData::RequestCharacterList { service_account_id } => {
                let config = get_config();

                let world_name;
                {
                    let mut game_data = self.gamedata.lock().unwrap();
                    world_name = game_data
                        .get_world_name(config.world.world_id)
                        .expect("Couldn't read world name");
                }

                let characters;
                {
                    let mut game_data = self.gamedata.lock().unwrap();

                    characters = self.database.get_character_list(
                        *service_account_id,
                        config.world.world_id,
                        &world_name,
                        &mut game_data,
                    );
                }

                // send response
                {
                    self.send_custom_response(PacketSegment {
                        segment_type: SegmentType::KawariIpc,
                        data: SegmentData::KawariIpc(CustomIpcSegment::new(
                            CustomIpcData::RequestCharacterListRepsonse { characters },
                        )),
                        ..Default::default()
                    })
                    .await;
                }
            }
            CustomIpcData::DeleteCharacter { content_id } => {
                self.database.delete_character(*content_id);

                // send response
                {
                    self.send_custom_response(PacketSegment {
                        segment_type: SegmentType::KawariIpc,
                        data: SegmentData::KawariIpc(CustomIpcSegment::new(
                            CustomIpcData::CharacterDeleted { deleted: 1 },
                        )),
                        ..Default::default()
                    })
                    .await;
                }
            }
            CustomIpcData::ImportCharacter {
                service_account_id,
                path,
            } => {
                let message;

                if let Ok(mut game_data) = self.gamedata.lock() {
                    if let Err(err) =
                        self.database
                            .import_character(&mut game_data, *service_account_id, path)
                    {
                        message = err.to_string();
                    } else {
                        message = "Successfully imported!".to_string();
                    }
                } else {
                    message = "Failed to read game data".to_string();
                }

                // send response
                {
                    self.send_custom_response(PacketSegment {
                        segment_type: SegmentType::KawariIpc,
                        data: SegmentData::KawariIpc(CustomIpcSegment::new(
                            CustomIpcData::CharacterImported { message },
                        )),
                        ..Default::default()
                    })
                    .await;
                }
            }
            CustomIpcData::RemakeCharacter {
                content_id,
                chara_make_json,
            } => {
                // overwrite it in the database
                self.database.set_chara_make(*content_id, chara_make_json);

                // reset flag
                self.database.set_remake_mode(*content_id, RemakeMode::None);

                // send response
                {
                    self.send_custom_response(PacketSegment {
                        segment_type: SegmentType::KawariIpc,
                        data: SegmentData::KawariIpc(CustomIpcSegment::new(
                            CustomIpcData::CharacterRemade {
                                content_id: *content_id,
                            },
                        )),
                        ..Default::default()
                    })
                    .await;
                }
            }
            _ => {
                panic!("The server is recieving a response or unknown custom IPC! {data:#?}")
            }
        }
    }
}
