use crate::{
    common::{
        determine_initial_starting_zone,
        workdefinitions::{CharaMake, RemakeMode},
    },
    config::get_config,
    inventory::Inventory,
    ipc::kawari::{CustomIpcData, CustomIpcSegment},
    packet::{PacketSegment, SegmentData, SegmentType},
};

use super::ZoneConnection;

pub async fn handle_custom_ipc(connection: &mut ZoneConnection, data: &CustomIpcSegment) {
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
                let mut game_data = connection.gamedata.lock().unwrap();

                city_state = game_data
                    .get_citystate(chara_make.classjob_id as u16)
                    .expect("Unknown citystate");
            }

            let mut inventory = Inventory::default();
            let (content_id, actor_id);
            {
                let mut game_data = connection.gamedata.lock().unwrap();

                inventory.equip_classjob_items(chara_make.classjob_id as u16, &mut game_data);

                // fill inventory
                inventory.equip_racial_items(
                    chara_make.customize.race,
                    chara_make.customize.gender,
                    &mut game_data,
                );

                (content_id, actor_id) = connection.database.create_player_data(
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
                connection
                    .send_custom_response(PacketSegment {
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
            let actor_id = connection.database.find_actor_id(*content_id);

            tracing::info!("We found an actor id: {actor_id}");

            // send them the actor id
            {
                connection
                    .send_custom_response(PacketSegment {
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
            let is_name_free = connection.database.check_is_name_free(name);

            // send response
            {
                connection
                    .send_custom_response(PacketSegment {
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
                let mut game_data = connection.gamedata.lock().unwrap();
                world_name = game_data
                    .get_world_name(config.world.world_id)
                    .expect("Couldn't read world name");
            }

            let characters;
            {
                let mut game_data = connection.gamedata.lock().unwrap();

                characters = connection.database.get_character_list(
                    *service_account_id,
                    config.world.world_id,
                    &world_name,
                    &mut game_data,
                );
            }

            // send response
            {
                connection
                    .send_custom_response(PacketSegment {
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
            connection.database.delete_character(*content_id);

            // send response
            {
                connection
                    .send_custom_response(PacketSegment {
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

            if let Ok(mut game_data) = connection.gamedata.lock() {
                if let Err(err) =
                    connection
                        .database
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
                connection
                    .send_custom_response(PacketSegment {
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
            connection
                .database
                .set_chara_make(*content_id, chara_make_json);

            // reset flag
            connection
                .database
                .set_remake_mode(*content_id, RemakeMode::None);

            // send response
            {
                connection
                    .send_custom_response(PacketSegment {
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
            panic!("The server is recieving a response or unknown custom IPC!")
        }
    }
}
