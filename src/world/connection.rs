use tokio::net::TcpStream;

use crate::{
    common::{Position, timestamp_secs},
    packet::{
        CompressionType, ConnectionType, PacketSegment, PacketState, SegmentType, parse_packet,
        send_packet,
    },
};

use super::{
    Inventory, Item, Zone,
    ipc::{
        ActorSetPos, ClientZoneIpcSegment, ContainerInfo, ContainerType, InitZone, ItemInfo,
        ServerZoneIpcData, ServerZoneIpcSegment, ServerZoneIpcType, UpdateClassInfo, WeatherChange,
    },
};

#[derive(Debug, Default)]
pub struct PlayerData {
    pub actor_id: u32,
    pub content_id: u64,
    pub account_id: u32,
}

/// Represents a single connection between an instance of the client and the world server
pub struct ZoneConnection {
    pub socket: TcpStream,

    pub state: PacketState,
    pub player_data: PlayerData,

    pub zone: Option<Zone>,
    pub spawn_index: u8,

    pub position: Position,
    pub inventory: Inventory,
}

impl ZoneConnection {
    pub async fn parse_packet(
        &mut self,
        data: &[u8],
    ) -> (Vec<PacketSegment<ClientZoneIpcSegment>>, ConnectionType) {
        parse_packet(data, &mut self.state).await
    }

    pub async fn send_segment(&mut self, segment: PacketSegment<ServerZoneIpcSegment>) {
        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Zone,
            CompressionType::Oodle,
            &[segment],
        )
        .await;
    }

    pub async fn set_player_position(&mut self, position: Position) {
        // set pos
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::ActorSetPos,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::ActorSetPos(ActorSetPos {
                    unk: 0x020fa3b8,
                    position,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }
    }

    pub async fn change_zone(&mut self, new_zone_id: u16) {
        self.zone = Some(Zone::load(new_zone_id));

        // Player Class Info
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::UpdateClassInfo,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo {
                    class_id: 35,
                    unknown: 1,
                    synced_level: 90,
                    class_level: 90,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // link shell information
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::LinkShellInformation,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::LinkShellInformation { unk: [0; 456] },
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // TODO: send unk16?

        // Init Zone
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::InitZone,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::InitZone(InitZone {
                    server_id: 0,
                    zone_id: self.zone.as_ref().unwrap().id,
                    weather_id: 1,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }
    }

    pub async fn change_weather(&mut self, new_weather_id: u16) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::WeatherChange,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::WeatherChange(WeatherChange {
                weather_id: new_weather_id,
                transistion_time: 1.0,
            }),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }

    pub fn get_free_spawn_index(&mut self) -> u8 {
        self.spawn_index += 1;
        self.spawn_index
    }

    pub async fn send_inventory(&mut self) {
        // item list
        {
            let equipped = self.inventory.equipped;

            let mut send_slot = async |slot_index: u16, item: &Item| {
                let ipc = ServerZoneIpcSegment {
                    op_code: ServerZoneIpcType::ItemInfo,
                    timestamp: timestamp_secs(),
                    data: ServerZoneIpcData::ItemInfo(ItemInfo {
                        container: ContainerType::Equipped,
                        slot: slot_index,
                        quantity: item.quantity,
                        catalog_id: item.id,
                        condition: 30000,
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                self.send_segment(PacketSegment {
                    source_actor: self.player_data.actor_id,
                    target_actor: self.player_data.actor_id,
                    segment_type: SegmentType::Ipc { data: ipc },
                })
                .await;
            };

            send_slot(0, &equipped.main_hand).await;
            send_slot(1, &equipped.off_hand).await;
            send_slot(2, &equipped.head).await;
            send_slot(3, &equipped.body).await;
            send_slot(4, &equipped.hands).await;
            send_slot(6, &equipped.legs).await;
            send_slot(7, &equipped.feet).await;
            send_slot(8, &equipped.ears).await;
            send_slot(9, &equipped.neck).await;
            send_slot(10, &equipped.wrists).await;
            send_slot(11, &equipped.right_ring).await;
            send_slot(12, &equipped.left_ring).await;
            send_slot(13, &equipped.soul_crystal).await;
        }

        // inform the client they have items equipped
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::ContainerInfo,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::ContainerInfo(ContainerInfo {
                    container: ContainerType::Equipped,
                    num_items: self.inventory.equipped.num_items(),
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }
    }
}
