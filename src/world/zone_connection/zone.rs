//! All things zone related, such as changing the weather or warping.

use mlua::Function;

use crate::{
    common::{Position, timestamp_secs},
    config::get_config,
    constants::OBFUSCATION_ENABLED_MODE,
    inventory::BuyBackList,
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, InitZone, InitZoneFlags, ServerZoneIpcData,
        ServerZoneIpcSegment, Warp, WeatherChange,
    },
    packet::{ConnectionState, PacketSegment, ScramblerKeyGenerator, SegmentData, SegmentType},
    world::{
        ObsfucationData, TeleportReason, ToServer, ZoneConnection,
        lua::{ExtraLuaState, LuaPlayer},
    },
};

impl ZoneConnection {
    /// Request the global server state to change our zone.
    pub async fn change_zone(&mut self, new_zone_id: u16) {
        self.player_data.teleport_reason = TeleportReason::NotSpecified;
        self.handle
            .send(ToServer::ChangeZone(
                self.id,
                self.player_data.actor_id,
                new_zone_id,
            ))
            .await;
    }

    /// Handle the zone change information from the server state.
    pub async fn handle_zone_change(
        &mut self,
        new_zone_id: u16,
        weather_id: u16,
        exit_position: Position,
        exit_rotation: f32,
        initial_login: bool,
    ) {
        // fade in?
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PrepareZoning {
                log_message: 0,
                target_zone: self.player_data.zone_id,
                animation: 0,
                param4: 0,
                hide_character: 0,
                fade_out: 1,
                param_7: 1,
                fade_out_time: 1,
                unk1: 8,
                unk2: 0,
            });
            self.send_ipc_self(ipc).await;
        }

        // If we are already in the same zone, we can do a Warp instead!
        if self.player_data.zone_id == new_zone_id && !initial_login {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Warp(Warp {
                dir: exit_rotation,
                position: exit_position,
                warp_type: 4, // for teleporting
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
            return;
        }

        self.player_data.zone_id = new_zone_id;
        self.exit_position = Some(exit_position);
        self.exit_rotation = Some(exit_rotation);

        // Player Class Info
        self.update_class_info().await;

        // Generate obsfucation-related keys if needed.
        if self.config.enable_packet_obsfucation {
            let seed1 = fastrand::u8(..);
            let seed2 = fastrand::u8(..);
            let seed3 = fastrand::u32(..);

            let generator = ScramblerKeyGenerator::new();

            self.obsfucation_data = ObsfucationData {
                seed1,
                seed2,
                seed3,
            };

            let ConnectionState::Zone { scrambler_keys, .. } = &mut self.state else {
                panic!("Unexpected connection type!");
            };
            *scrambler_keys = Some(generator.generate(seed1, seed2, seed3));

            tracing::info!(
                "You enabled packet obsfucation in your World config, if things break please report it!",
            );
        }

        // they send the initialize packet again for some reason
        {
            self.send_segment(PacketSegment {
                segment_type: SegmentType::Initialize,
                data: SegmentData::Initialize {
                    actor_id: self.player_data.actor_id,
                    timestamp: timestamp_secs(),
                },
                ..Default::default()
            })
            .await;
        }

        // Clear the server's copy of the buyback list.
        self.player_data.buyback_list = BuyBackList::default();

        // Init Zone
        {
            let config = get_config();

            let extra_flags = if initial_login {
                InitZoneFlags::INITIAL_LOGIN
            } else {
                InitZoneFlags::default()
            };

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InitZone(InitZone {
                territory_type: new_zone_id,
                weather_id: weather_id as u8,
                flags: InitZoneFlags::ENABLE_FLYING | InitZoneFlags::HIDE_SERVER | extra_flags,
                obsfucation_mode: if config.world.enable_packet_obsfucation {
                    OBFUSCATION_ENABLED_MODE
                } else {
                    0
                },
                seed1: !self.obsfucation_data.seed1,
                seed2: !self.obsfucation_data.seed2,
                seed3: !self.obsfucation_data.seed3,
                festivals_id1: config.world.active_festivals,
                festivals_id2: config.world.active_festivals,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
        }

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::SetItemLevel {
                level: self.player_data.inventory.equipped.calculate_item_level() as u32,
            },
        })
        .await;

        // send some weird thing to make the zone load correctly
        {
            self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::UnkZoneLoad1 {
                unk1: [0; 56],
            }))
            .await;

            self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::UnkZoneLoad2 {
                unk1: [0; 8],
            }))
            .await;
        }
    }

    pub async fn warp(&mut self, warp_id: u32) {
        self.player_data.teleport_reason = TeleportReason::NotSpecified;
        self.handle
            .send(ToServer::Warp(self.id, self.player_data.actor_id, warp_id))
            .await;
    }

    pub async fn warp_aetheryte(&mut self, aetheryte_id: u32) {
        self.player_data.teleport_reason = TeleportReason::Aetheryte;
        self.handle
            .send(ToServer::WarpAetheryte(
                self.id,
                self.player_data.actor_id,
                aetheryte_id,
            ))
            .await;
    }

    pub async fn change_weather(&mut self, new_weather_id: u16) {
        self.weather_id = new_weather_id;

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::WeatherId(WeatherChange {
            weather_id: new_weather_id,
            transistion_time: 1.0,
        }));
        self.send_ipc_self(ipc).await;
    }

    pub async fn spawn_eobjs(&mut self, lua_player: &mut LuaPlayer) {
        let lua = self.lua.lock();
        let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

        let key = self.player_data.zone_id as u32;
        if let Some(zone_eobj_script) = state.zone_eobj_scripts.get(&key) {
            lua.scope(|scope| {
                let connection_data = scope
                    .create_userdata_ref_mut(&mut lua_player.zone_data)
                    .unwrap();

                let config = get_config();

                let file_name = format!("{}/{}", &config.world.scripts_location, zone_eobj_script);
                lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
                    .set_name("@".to_string() + &file_name)
                    .exec()
                    .unwrap();

                let func: Function = lua.globals().get("onRequestEObjSpawn").unwrap();

                func.call::<()>(connection_data).unwrap();

                Ok(())
            })
            .unwrap();
        } else {
            tracing::info!(
                "Zone {} doesn't have an eobj script.",
                self.player_data.zone_id
            );
        }
    }
}
