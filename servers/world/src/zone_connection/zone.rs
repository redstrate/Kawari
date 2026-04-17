//! All things zone related, such as changing the weather or warping.

use crate::{
    ObsfucationData, TeleportReason, ToServer, ZoneConnection,
    inventory::BuyBackList,
    lua::{LuaContent, LuaZone},
};
use kawari::{
    common::{
        HandlerId, HandlerType, HouseId, HouseUnit, HousingFlag, LandData, Position, timestamp_secs,
    },
    config::get_config,
    constants::OBFUSCATION_ENABLED_MODE,
    ipc::zone::{
        ActorControlCategory, Condition, ContentRegistrationFlags, FurnitureList, House,
        HouseExterior, HouseList, HouseStatus, HousingInteriorDetails, PlotSize, ServerZoneIpcData,
        ServerZoneIpcSegment, WarpType, WeatherChange, ZoneInit, ZoneInitFlags,
    },
    packet::{ConnectionState, PacketSegment, ScramblerKeyGenerator, SegmentData, SegmentType},
};
use physis::TerritoryIntendedUse;

impl ZoneConnection {
    /// Request the global server state to change our zone.
    pub async fn change_zone(
        &mut self,
        new_zone_id: u16,
        new_position: Option<Position>,
        new_rotation: Option<f32>,
        warp_type_info: Option<(WarpType, u8, u8, u8)>,
    ) {
        self.teleport_reason = TeleportReason::NotSpecified;
        self.handle
            .send(ToServer::ChangeZone(
                self.id,
                self.player_data.character.actor_id,
                new_zone_id,
                new_position,
                new_rotation,
                warp_type_info,
            ))
            .await;
    }

    /// Handle the zone change information from the server state.
    pub async fn handle_zone_change(
        &mut self,
        new_zone_id: u16,
        content_finder_condition_id: u16,
        weather_id: u16,
        exit_position: Position,
        exit_rotation: f32,
        initial_login: bool,
        director_vars: Option<ServerZoneIpcSegment>,
        lua_zone: &LuaZone,
        lua_content: &mut LuaContent,
    ) {
        self.spawned_in = false;

        let bound_by_duty = content_finder_condition_id != 0;

        // Commit back our zone id and other volatile info on zone change.
        {
            self.player_data.volatile.zone_id = new_zone_id as i32;
            self.player_data.volatile.position = exit_position;
            self.player_data.volatile.rotation = exit_rotation as f64;

            let mut db = self.database.lock();
            db.commit_volatile(&self.player_data);
        }

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
                    actor_id: self.player_data.character.actor_id,
                    timestamp: timestamp_secs(),
                },
                ..Default::default()
            })
            .await;
        }

        // Clear the server's copy of the buyback list.
        self.player_data.buyback_list = BuyBackList::default();

        let config = get_config();

        // Send obsfucation init
        if config.world.enable_packet_obsfucation {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InitializeObfuscation {
                unk_before: [0; 6],
                obsfucation_mode: OBFUSCATION_ENABLED_MODE,
                seed1: !self.obsfucation_data.seed1,
                seed2: !self.obsfucation_data.seed2,
                seed3: !self.obsfucation_data.seed3,
            });
            self.send_ipc_self(ipc).await;
        }

        // Send owned housing list (unsure where this fits in before ZoneInit?!)
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::OwnedHousing {
                unk1: Default::default(),
                unk2: Default::default(),
                unk3: Default::default(),
                unk4: Default::default(),
                unk5: Default::default(),
                apartment: LandData {
                    id: HouseId {
                        unit: HouseUnit {
                            apartment_division_plot_index: 0,
                            apartment_flag: true,
                        },
                        unk1: 0,
                        ward_index: 0,
                        room_number: 1,
                        territory_type_id: 340,
                        world_id: config.world.world_id,
                    },
                    flags: 19,
                    unk1: 0,
                },
            });
            self.send_ipc_self(ipc).await;
        }

        // Send the list of available items if we're in an inn, since its only accessible via their beds.
        if lua_zone.intended_use == TerritoryIntendedUse::Inn as u8 {
            let display_ids;
            {
                let mut gamedata = self.gamedata.lock();
                display_ids = gamedata.get_latest_fittingshop_display_ids();
            }

            let ipc =
                ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateFittingShop { display_ids });
            self.send_ipc_self(ipc).await;
        }

        // Init Zone
        {
            let mut flags = if initial_login {
                ZoneInitFlags::INITIAL_LOGIN
            } else if bound_by_duty {
                ZoneInitFlags::UNK1 | ZoneInitFlags::UNK3
            } else {
                ZoneInitFlags::default()
            };

            if !bound_by_duty {
                flags |= ZoneInitFlags::ENABLE_FLYING;
            }

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ZoneInit(ZoneInit {
                territory_type: new_zone_id,
                weather_id: weather_id as u8,
                flags,
                content_finder_condition_id,
                game_festival_ids: config.world.active_festivals,
                ui_festival_ids: config.world.active_festivals,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
        }

        let level;
        {
            let mut game_data = self.gamedata.lock();

            level = self
                .player_data
                .inventory
                .equipped
                .calculate_item_level(&mut game_data) as u32;
        }

        self.actor_control_self(ActorControlCategory::SetItemLevel { level })
            .await;

        // send some weird thing to make the zone load correctly
        if !bound_by_duty {
            self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::DailyQuests {
                unk1: [0; 56],
            }))
            .await;

            self.send_ipc_self(ServerZoneIpcSegment::new(
                ServerZoneIpcData::DailyQuestRepeatFlags { unk1: [0; 8] },
            ))
            .await;
        }

        if initial_login {
            self.send_quest_information().await;
        }

        if lua_zone.intended_use == TerritoryIntendedUse::HousingOutdoor as u8 {
            let mut houses = [House::default(); 30];

            // First, populate the houses in this ward. Note that for now, we treat every ward the same.
            // TODO: For now, we hardcode 3 prefab houses as demonstration units until we implement more of the system. One cottage, one house and one mansion, all set to be individually owned and locked. Plots 5, 6, and 12 were chosen due to their close proximity to each other.
            // Glade house (Wood)
            houses[4] = House {
                plot_size: PlotSize::Medium,
                status: HouseStatus::HouseBuilt,
                flags: HousingFlag::OPEN,
                exterior: HouseExterior {
                    roof_id: 1029,
                    walls_id: 3589,
                    windows_id: 2562,
                    door_id: 514,
                    ..Default::default()
                },
                ..Default::default()
            };

            // Hingan mansion (Mokuzo)
            houses[5] = House {
                plot_size: PlotSize::Large,
                status: HouseStatus::HouseBuilt,
                flags: HousingFlag::OPEN,
                exterior: HouseExterior {
                    roof_id: 1081,
                    walls_id: 3632,
                    windows_id: 2579,
                    door_id: 531,
                    ..Default::default()
                },
                ..Default::default()
            };

            // Highland cottage (Wood)
            houses[11] = House {
                plot_size: PlotSize::Small,
                status: HouseStatus::HouseBuilt,
                flags: HousingFlag::OPEN | HousingFlag::OWNED_BY_FC,
                exterior: HouseExterior {
                    roof_id: 1136,
                    walls_id: 3687,
                    windows_id: 2598,
                    door_id: 550,
                    ..Default::default()
                },
                ..Default::default()
            };

            let config = get_config();
            self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::HouseList(
                HouseList {
                    land_id: 0,
                    ward: 0,
                    territory_type_id: lua_zone.zone_id,
                    world_id: config.world.world_id,
                    subdivision: 257, // TODO: Figure out more about subdivisions
                    houses,
                },
            )))
            .await;

            // Finally, populate the exterior furniture.
            // TODO: Actually send some real furniture, once we can do that!
            for index in 0..8 {
                self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::FurnitureList(
                    FurnitureList {
                        count: 8,
                        index,
                        ..Default::default()
                    },
                )))
                .await;
            }
        }

        if lua_zone.intended_use == TerritoryIntendedUse::HousingIndoor as u8 {
            let config = get_config();
            // Bare minimum stuff to make housing interiors load
            self.send_ipc_self(ServerZoneIpcSegment::new(
                ServerZoneIpcData::HousingInteriorDetails(HousingInteriorDetails::default()),
            ))
            .await;

            // The LandId is currently set so that plugins like HousingPos/Buildingway can plop stuff down
            self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::FurnitureList(
                FurnitureList {
                    id: HouseId {
                        unit: HouseUnit {
                            apartment_division_plot_index: 0,
                            apartment_flag: true,
                        },
                        unk1: 0,
                        room_number: 1,
                        ward_index: 0,
                        territory_type_id: 340,
                        world_id: config.world.world_id,
                    },
                    count: 1,
                    index: 0,
                    unk2: 100, // Indoors
                    ..Default::default()
                },
            )))
            .await;
        }

        self.conditions
            .toggle_condition(Condition::BoundByDuty, bound_by_duty);
        self.conditions
            .toggle_condition(Condition::BoundByDuty56, bound_by_duty);

        self.send_conditions().await;

        // Terminate the old director if we're exiting an instance
        if self.content_handler_id.0 != 0 && content_finder_condition_id == 0 {
            self.actor_control_self(ActorControlCategory::TerminateDirector {
                handler_id: self.content_handler_id,
            })
            .await;

            self.content_handler_id = HandlerId::default();
            self.current_instance_id = None;
        }

        self.synced_level = None;

        // Initialize director as needed
        if let Some(intended_use) = TerritoryIntendedUse::from_repr(lua_zone.intended_use) {
            if let Some(director_type) = HandlerType::from_intended_use(intended_use) {
                let content_id = if bound_by_duty {
                    let mut game_data = self.gamedata.lock();
                    game_data
                        .find_content_for_content_finder_id(content_finder_condition_id)
                        .unwrap()
                } else {
                    // There is no content associated with FATE directors.
                    if director_type.requires_content_id() {
                        tracing::warn!(
                            "Failed to find content ID for {content_finder_condition_id}?"
                        );
                    }
                    0xFFFF
                };

                // TODO: this needs to be networked
                let needs_sync = {
                    if self
                        .content_settings
                        .unwrap_or_default()
                        .contains(ContentRegistrationFlags::UNRESTRICTED_PARTY)
                    {
                        self.content_settings
                            .unwrap_or_default()
                            .contains(ContentRegistrationFlags::LEVEL_SYNC)
                    } else {
                        !self
                            .content_settings
                            .unwrap_or_default()
                            .contains(ContentRegistrationFlags::EXPLORER_MODE)
                    }
                };

                if needs_sync {
                    let synced_level;
                    let current_level;
                    {
                        let mut game_data = self.gamedata.lock();
                        synced_level =
                            game_data.find_content_synced_level(content_finder_condition_id);
                        current_level = self.current_level(&game_data);
                    }

                    self.synced_level = None;
                    if let Some(synced_level) = synced_level
                        && current_level > synced_level as u16
                    {
                        self.synced_level = Some(synced_level);
                    }
                }

                self.current_instance_id = Some(content_id);

                let director_id = HandlerId::new(director_type, content_id);
                tracing::info!("Initializing director {director_id}...");

                {
                    let mut game_data = self.gamedata.lock();
                    lua_content.duration = game_data
                        .find_content_time_limit(content_id)
                        .unwrap_or_default()
                        * 60;
                }

                let flags = if self
                    .content_settings
                    .unwrap_or_default()
                    .contains(ContentRegistrationFlags::EXPLORER_MODE)
                    && director_type.requires_content_id()
                {
                    1
                } else {
                    0
                };

                // Initialize the content director
                self.actor_control_self(ActorControlCategory::InitDirector {
                    handler_id: director_id,
                    content_id,
                    flags,
                })
                .await;

                if director_type.requires_content_id() {
                    if let Some(director_vars) = director_vars {
                        self.send_ipc_self(director_vars).await;
                    }

                    self.send_ipc_self(ServerZoneIpcSegment::new(
                        ServerZoneIpcData::UnkDirector1 {
                            unk: [
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            ],
                        },
                    ))
                    .await;

                    self.content_handler_id = director_id;
                }
            } else {
                tracing::warn!("{intended_use} does not have a known director type yet?");
            }
        } else {
            tracing::warn!("Unknown TerritoryIntendedUse: {}!", lua_zone.intended_use);
        }

        // Player Class Info
        // NOTE: It's important it happens after we set our synced level!
        self.update_class_info().await;
        self.send_stats().await;
    }

    pub async fn warp(&mut self, warp_id: u32) {
        self.teleport_reason = TeleportReason::NotSpecified;
        self.handle
            .send(ToServer::Warp(
                self.id,
                self.player_data.character.actor_id,
                warp_id,
            ))
            .await;
    }

    pub async fn warp_aetheryte(&mut self, aetheryte_id: u32, housing_aethernet: bool) {
        self.teleport_reason = TeleportReason::Aetheryte;
        self.handle
            .send(ToServer::WarpAetheryte(
                self.id,
                self.player_data.character.actor_id,
                aetheryte_id,
                housing_aethernet,
            ))
            .await;
    }

    pub async fn change_weather(&mut self, new_weather_id: u16) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::WeatherId(WeatherChange {
            weather_id: new_weather_id,
            transistion_time: 1.0,
        }));
        self.send_ipc_self(ipc).await;
    }

    pub async fn discover_location(&mut self, map_id: u32, map_part_id: u32) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LocationDiscovered {
            map_id,
            map_part_id,
        });
        self.send_ipc_self(ipc).await;
    }

    pub async fn join_content(&mut self, id: u16) {
        // Store our old information, for when we leave the instance
        self.old_zone_id = self.player_data.volatile.zone_id as u16;
        self.old_position = self.player_data.volatile.position;
        self.old_rotation = self.player_data.volatile.rotation as f32;

        self.handle
            .send(ToServer::JoinContent(
                self.id,
                self.player_data.character.actor_id,
                id,
            ))
            .await;
    }

    /// Ensure the player is placed in a valid zone, and if they aren't they are teleported back to their homepoint.
    pub async fn ensure_valid_zone(&mut self) {
        let zone_id = self.player_data.volatile.zone_id;

        // If the player isn't in a valid zone, or in instanced content (both crash the game) then we need to reset them.
        let should_reset;
        {
            let mut game_data = self.gamedata.lock();
            should_reset = !game_data.is_zone_valid(zone_id as u16)
                || game_data.is_zone_associated_with_content(zone_id as u16);
        }
        if should_reset {
            // TODO: teleport them to their homepoint instead
            self.player_data.volatile.zone_id = 132;
            self.player_data.volatile.position = Position::default();

            self.send_notice("Moved you to a safe area to prevent a crash!")
                .await;
        }
    }
}
