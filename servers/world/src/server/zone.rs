use std::{collections::HashMap, sync::Arc, time::Duration};

use parking_lot::Mutex;
use physis::{
    layer::{
        ExitRangeInstanceObject, InstanceObject, LayerEntryData, LayerGroup,
        PopRangeInstanceObject, Transformation, TriggerBoxShape,
    },
    lvb::Lvb,
    resource::Resource,
};

use crate::{
    ClientId, FromServer, StatusEffects, ToServer,
    lua::LuaZone,
    server::{
        NetworkedActor, WorldServer,
        instance::Instance,
        network::{DestinationNetwork, NetworkState},
    },
    zone_connection::TeleportQuery,
};
use kawari::{
    common::{
        DistanceRange, EventHandlerType, GameData, INVALID_OBJECT_ID, ObjectId, Position,
        TerritoryNameKind, euler_to_direction,
    },
    ipc::zone::{ActorControl, ActorControlCategory, ActorControlSelf, Conditions, ObjectSpawn},
};

#[derive(Debug)]
pub enum MapGimmick {
    Jump {
        /// The position to land on.
        to_position: Position,
        /// The GimmickJump type.
        gimmick_jump_type: u32,
        /// The animation ID to play for the EObj.
        sgb_animation_id: u32,
        /// The EObj's instance ID to play the animation for.
        eobj_instance_id: u32,
    },
}

/// Simpler form of a MapRange object designed for collision detection.
#[derive(Debug)]
pub struct MapRange {
    /// Trigger box shape.
    pub trigger_box_shape: TriggerBoxShape,
    /// Position of this range in the world.
    pub position: Position,
    /// Relative scale of this range.
    pub scale: [f32; 3],
    /// Whether this map range represents a sanctuary.
    pub sanctuary: bool,
    /// Whether this map range represents a PvP duel area.
    pub duel: bool,
    /// Whether this map range represents a gimmick, like a jumping pad.
    pub gimmick: Option<MapGimmick>,
    /// Game Object ID.
    pub instance_id: u32,
}

/// Represents a loaded zone
#[derive(Default, Debug)]
pub struct Zone {
    pub id: u16,
    pub internal_name: String,
    pub region_name: String,
    pub place_name: String,
    pub intended_use: u8,
    pub layer_groups: Vec<LayerGroup>,
    pub navimesh_path: String,
    pub map_id: u16,
    cached_npc_base_ids: HashMap<u32, u32>,
    map_ranges: Vec<MapRange>,
}

impl Zone {
    pub fn load(game_data: &mut GameData, id: u16) -> Self {
        let mut zone = Self {
            id,
            ..Default::default()
        };

        let Some(row) = game_data.territory_type_sheet.get_row(id as u32) else {
            tracing::warn!("Invalid zone id {id}, allowing anyway...");
            return zone;
        };

        zone.intended_use = *row.TerritoryIntendedUse().into_u8().unwrap();
        zone.map_id = *row.Map().into_u16().unwrap();

        // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
        let bg_path = row.Bg().into_string().unwrap();
        if bg_path.is_empty() {
            tracing::warn!("Invalid zone id {id}, allowing anyway...");
            return zone;
        }

        let path = format!("bg/{}.lvb", &bg_path);
        tracing::info!("Loading {}", path);
        if let Some(lgb_file) = game_data.resource.read(&path) {
            let lgb = Lvb::from_existing(&lgb_file).unwrap();

            let mut load_lgb = |path: &str| -> Option<LayerGroup> {
                // Skip LGBs that aren't relevant for the server
                if path.ends_with("bg.lgb")
                    || path.ends_with("vfx.lgb")
                    || path.ends_with("sound.lgb")
                {
                    return None;
                }

                let lgb_file = game_data.resource.read(path)?;
                tracing::info!("Loading {path}");
                let lgb = LayerGroup::from_existing(&lgb_file);
                if lgb.is_none() {
                    tracing::warn!(
                        "Failed to parse {path}, this is most likely a bug in Physis and should be reported somewhere!"
                    )
                }
                lgb
            };

            for path in &lgb.scns[0].header.path_layer_group_resources {
                if let Some(lgb) = load_lgb(path) {
                    zone.layer_groups.push(lgb);
                }
            }

            for layer_set in &lgb.scns[0].unk3.unk2 {
                // FIXME: this is wrong. I think there might be multiple, separate navimeshes in really big zones but I'm not sure yet.
                zone.navimesh_path = layer_set.path_nvm.replace("/server/data/", "").to_string();
            }
        }

        // create NPC ID cache
        for layer_group in &zone.layer_groups {
            for chunk in &layer_group.chunks {
                for layer in &chunk.layers {
                    for object in &layer.objects {
                        if let LayerEntryData::EventNPC(npc) = &object.data {
                            zone.cached_npc_base_ids
                                .insert(object.instance_id, npc.parent_data.parent_data.base_id);
                        }
                        if let LayerEntryData::MapRange(map_range) = &object.data {
                            zone.map_ranges.push(MapRange {
                                trigger_box_shape: map_range.parent_data.trigger_box_shape,
                                position: Position {
                                    x: object.transform.translation[0],
                                    y: object.transform.translation[1],
                                    z: object.transform.translation[2],
                                },
                                scale: object.transform.scale,
                                sanctuary: map_range.rest_bonus_enabled,
                                duel: false,
                                gimmick: None,
                                instance_id: object.instance_id,
                            });
                        }
                        if let LayerEntryData::EventRange(event_range) = &object.data {
                            zone.map_ranges.push(MapRange {
                                trigger_box_shape: event_range.parent_data.trigger_box_shape,
                                position: Position {
                                    x: object.transform.translation[0],
                                    y: object.transform.translation[1],
                                    z: object.transform.translation[2],
                                },
                                scale: object.transform.scale,
                                sanctuary: false,
                                // This is guesswork since there's only one dueling location in-game
                                duel: event_range.unk_flags[0] == 1
                                    && event_range.unk_flags[3] == 1
                                    && event_range.unk_flags[4] == 1
                                    && event_range.unk_flags[5] == 1,
                                gimmick: None,
                                instance_id: object.instance_id,
                            });
                        }
                    }

                    // Second pass for eobjs
                    for object in &layer.objects {
                        if let LayerEntryData::EventObject(eobj) = &object.data {
                            let eobj_data = game_data.get_eobj_data(eobj.parent_data.base_id);
                            let event_type = EventHandlerType::from_repr(eobj_data >> 16);

                            if let Some(EventHandlerType::GimmickRect) = event_type {
                                // GimmickRects are used for stuff like the Golden Saucer jumping pads, and is handled server-side.
                                // Thus, we need to go through and mark these MapRanges to play said event.
                                if let Some(gimmick_rect_info) =
                                    game_data.get_gimmick_rect_info(eobj_data & 0xFFF)
                                    && let Some(target_pop_range) =
                                        zone.find_pop_range(gimmick_rect_info.params[1])
                                {
                                    let gimmick_jump_type = gimmick_rect_info.params[0];
                                    let target_event_range = gimmick_rect_info.layout_id;
                                    let sgb_animation_id = gimmick_rect_info.params[2];

                                    // 8 seems to indicate a jumping pad
                                    if gimmick_rect_info.trigger_in == 8 {
                                        let map_gimmick = MapGimmick::Jump {
                                            to_position: Position {
                                                x: target_pop_range.0.transform.translation[0],
                                                y: target_pop_range.0.transform.translation[1],
                                                z: target_pop_range.0.transform.translation[2],
                                            },
                                            gimmick_jump_type,
                                            sgb_animation_id,
                                            eobj_instance_id: object.instance_id,
                                        };

                                        for map_range in &mut zone.map_ranges {
                                            if map_range.instance_id == target_event_range {
                                                map_range.gimmick = Some(map_gimmick);
                                                break;
                                            }
                                        }
                                    } else {
                                        tracing::warn!(
                                            "Unsupported Gimmick trigger {}",
                                            gimmick_rect_info.trigger_in
                                        );
                                    }
                                } else {
                                    tracing::warn!(
                                        "Failed to lookup Gimmick {}?!",
                                        eobj_data & 0xFFF
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // load names
        let fallback = "<Unable to load name!>";
        zone.internal_name = game_data
            .get_territory_name(id as u32, TerritoryNameKind::Internal)
            .unwrap_or(fallback.to_string());
        zone.region_name = game_data
            .get_territory_name(id as u32, TerritoryNameKind::Region)
            .unwrap_or(fallback.to_string());
        zone.place_name = game_data
            .get_territory_name(id as u32, TerritoryNameKind::Place)
            .unwrap_or(fallback.to_string());

        zone
    }

    /// Search for an exit box matching an id.
    pub fn find_exit_box(
        &self,
        instance_id: u32,
    ) -> Option<(&InstanceObject, &ExitRangeInstanceObject)> {
        // TODO: also check position!
        for layer_group in &self.layer_groups {
            for layer in &layer_group.chunks[0].layers {
                for object in &layer.objects {
                    if let LayerEntryData::ExitRange(exit_range) = &object.data
                        && object.instance_id == instance_id
                    {
                        return Some((object, exit_range));
                    }
                }
            }
        }

        None
    }

    pub fn find_pop_range(
        &self,
        instance_id: u32,
    ) -> Option<(&InstanceObject, &PopRangeInstanceObject)> {
        // TODO: also check position!
        for layer_group in &self.layer_groups {
            for layer in &layer_group.chunks[0].layers {
                for object in &layer.objects {
                    if let LayerEntryData::PopRange(pop_range) = &object.data
                        && object.instance_id == instance_id
                    {
                        return Some((object, pop_range));
                    }
                }
            }
        }

        None
    }

    pub fn to_lua_zone(&self, weather_id: u16) -> LuaZone {
        LuaZone {
            zone_id: self.id,
            weather_id,
            internal_name: self.internal_name.clone(),
            region_name: self.region_name.clone(),
            place_name: self.place_name.clone(),
            intended_use: self.intended_use,
            map_id: self.map_id,
            cached_npc_base_ids: self.cached_npc_base_ids.clone(),
            ..Default::default()
        }
    }

    /// Tries to locate the dungeon entrance based on the game object ID.
    pub fn find_entrance(&self) -> Option<&InstanceObject> {
        // TODO: also check position!
        for layer_group in &self.layer_groups {
            for layer in &layer_group.chunks[0].layers {
                for object in &layer.objects {
                    if let LayerEntryData::EventObject(eobj) = &object.data
                        && eobj.parent_data.base_id == 2000182
                    {
                        return Some(object);
                    }
                }
            }
        }

        None
    }

    /// Returns a list of event objects to spawn by default.
    ///
    /// For example, the Gold Saucer arcade machines or shortcuts in dungeons.
    pub fn get_event_objects(&self, game_data: &mut GameData) -> Vec<ObjectSpawn> {
        let mut object_spawns = Vec::new();

        for layer_group in &self.layer_groups {
            for layer in &layer_group.chunks[0].layers {
                for object in &layer.objects {
                    if let LayerEntryData::EventObject(eobj) = &object.data {
                        // NOTE: this seems to keep the gold saucer machines, and not much else. needs more testing!
                        if game_data.get_eobj_pop_type(eobj.parent_data.base_id) != 1 {
                            continue;
                        }

                        let unselectable = if let Some(event_type) = EventHandlerType::from_repr(
                            game_data.get_eobj_data(eobj.parent_data.base_id) >> 16,
                        ) {
                            // Unsure if excluding certain types is the way to go, but let's see.
                            matches!(event_type, EventHandlerType::GimmickRect)
                        } else {
                            false // don't make selectable to be on the safe side.
                        };

                        object_spawns.push(ObjectSpawn {
                            kind: 7, // ObjectKind::EventObj
                            base_id: eobj.parent_data.base_id,
                            unselectable,
                            entity_id: ObjectId(fastrand::u32(..)),
                            layout_id: object.instance_id,
                            bind_layout_id: eobj.bound_instance_id,
                            owner_id: INVALID_OBJECT_ID,
                            radius: 1.0,
                            rotation: euler_to_direction(object.transform.rotation),
                            position: Position {
                                x: object.transform.translation[0],
                                y: object.transform.translation[1],
                                z: object.transform.translation[2],
                            },
                            ..Default::default()
                        });
                    }
                }
            }
        }

        object_spawns
    }

    /// Returns a list of MapRanges that overlap this position.
    pub fn get_overlapping_map_ranges(&self, position: Position) -> Vec<&MapRange> {
        let mut overlapping = Vec::new();

        for map_range in &self.map_ranges {
            match map_range.trigger_box_shape {
                TriggerBoxShape::Box => {
                    // TODO: support oriented boxes (this is used by sanctuary boundaries, for some reason)
                    let min_x = map_range.position.x - (map_range.scale[0]);
                    let max_x = map_range.position.x + (map_range.scale[0]);

                    let min_y = map_range.position.y - (map_range.scale[1]);
                    let max_y = map_range.position.y + (map_range.scale[1]);

                    let min_z = map_range.position.z - (map_range.scale[2]);
                    let max_z = map_range.position.z + (map_range.scale[2]);

                    if position.x >= min_x
                        && position.x <= max_x
                        && position.y >= min_y
                        && position.y <= max_y
                        && position.z >= min_z
                        && position.z <= max_z
                    {
                        overlapping.push(map_range);
                    }
                }
                TriggerBoxShape::Cylinder => {
                    // TODO: support arbitrarily-rotated cylinders
                    let length = map_range.scale[1] * 2.0;
                    let length_sq = f32::powi(length, 2);

                    let pt1 = Position {
                        x: map_range.position.x,
                        y: map_range.position.y - map_range.scale[1],
                        z: map_range.position.z,
                    };
                    let pt2 = Position {
                        x: map_range.position.x,
                        y: map_range.position.y + map_range.scale[1],
                        z: map_range.position.z,
                    };

                    let radius = map_range.scale[0]; // TODO: support individual radii (if that's even a thing, assert please)
                    let radius_sq = f32::powi(radius, 2);

                    if Self::cylinder_test(pt1, pt2, length_sq, radius_sq, position) != -1.0 {
                        overlapping.push(map_range);
                    }
                }
                _ => {} // TODO
            }
        }

        overlapping
    }

    // From https://www.flipcode.com/archives/Fast_Point-In-Cylinder_Test.shtml
    fn cylinder_test(
        pt1: Position,
        pt2: Position,
        length_sq: f32,
        radius_sq: f32,
        test_pt: Position,
    ) -> f32 {
        let dx = pt2.x - pt1.x;
        let dy = pt2.y - pt1.y;
        let dz = pt2.z - pt1.z;

        let pdx = test_pt.x - pt1.x;
        let pdy = test_pt.y - pt1.y;
        let pdz = test_pt.z - pt1.z;

        let dot = pdx * dx + pdy * dy + pdz * dz;
        if dot < 0.0 || dot > length_sq {
            -1.0
        } else {
            let dsq = (pdx * pdx + pdy * pdy + pdz * pdz) - dot * dot / length_sq;

            if dsq > radius_sq { -1.0 } else { dsq }
        }
    }
}

fn begin_change_zone<'a>(
    data: &'a mut WorldServer,
    network: &mut NetworkState,
    game_data: &mut GameData,
    destination_zone_id: u16,
    actor_id: ObjectId,
    from_id: ClientId,
) -> Option<&'a mut Instance> {
    // inform the players in this zone that this actor left
    if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
        current_instance.actors.remove(&actor_id);
        network.inform_remove_actor(current_instance, from_id, actor_id);
    }

    // then find or create a new instance with the zone id
    data.ensure_exists(destination_zone_id, game_data);
    data.find_instance_mut(destination_zone_id)
}

/// Sends the needed information to ZoneConnection for a zone change.
fn change_zone_warp_to_pop_range(
    data: &mut WorldServer,
    network: &mut NetworkState,
    game_data: &mut GameData,
    destination_zone_id: u16,
    destination_instance_id: u32,
    actor_id: ObjectId,
    from_id: ClientId,
) {
    let target_instance = begin_change_zone(
        data,
        network,
        game_data,
        destination_zone_id,
        actor_id,
        from_id,
    )
    .unwrap();

    target_instance.insert_empty_actor(actor_id);

    let exit_position;
    let exit_rotation;
    if let Some((destination_object, _)) =
        target_instance.zone.find_pop_range(destination_instance_id)
    {
        exit_position = Some(Position {
            x: destination_object.transform.translation[0],
            y: destination_object.transform.translation[1],
            z: destination_object.transform.translation[2],
        });
        exit_rotation = Some(euler_to_direction(destination_object.transform.rotation));
    } else {
        exit_position = None;
        exit_rotation = None;
    }

    do_change_zone(
        network,
        target_instance,
        destination_zone_id,
        exit_position,
        exit_rotation,
        from_id,
    );
}

/// Sends the needed information to ZoneConnection for a zone change.
pub fn change_zone_warp_to_entrance(
    network: &mut NetworkState,
    target_instance: &Instance,
    destination_zone_id: u16,
    from_id: ClientId,
) {
    let exit_position;
    let exit_rotation;
    if let Some(destination_object) = target_instance.zone.find_entrance() {
        exit_position = Some(Position {
            x: destination_object.transform.translation[0],
            y: destination_object.transform.translation[1],
            z: destination_object.transform.translation[2],
        });
        exit_rotation = Some(euler_to_direction(destination_object.transform.rotation));
    } else {
        exit_position = None;
        exit_rotation = None;
    }

    do_change_zone(
        network,
        target_instance,
        destination_zone_id,
        exit_position,
        exit_rotation,
        from_id,
    );
}

/// Sends the needed information to ZoneConnection for a zone change.
fn do_change_zone(
    network: &mut NetworkState,
    target_instance: &Instance,
    destination_zone_id: u16,
    exit_position: Option<Position>,
    exit_rotation: Option<f32>,
    from_id: ClientId,
) {
    // Clear spawn pools
    let state = network.get_state_mut(from_id).unwrap();
    state.actor_allocator.clear();
    state.object_allocator.clear();

    // now that we have all of the data needed, inform the connection of where they need to be
    let msg = FromServer::ChangeZone(
        destination_zone_id,
        target_instance.content_finder_condition_id,
        target_instance.weather_id,
        exit_position.unwrap_or_default(),
        exit_rotation.unwrap_or_default(),
        target_instance.zone.to_lua_zone(target_instance.weather_id),
        false,
    );
    network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
}

/// Process zone-related messages.
pub fn handle_zone_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    game_data: Arc<Mutex<GameData>>,
    msg: &ToServer,
) {
    match msg {
        ToServer::ZoneLoaded(from_id, from_actor_id, player_spawn) => {
            tracing::info!(
                "Client {from_id:?} has now loaded into the zone, sending them existing player data."
            );

            let mut data = data.lock();

            // replace the connection's actor in the table
            let instance = data.find_actor_instance_mut(*from_actor_id).unwrap();
            *instance.find_actor_mut(*from_actor_id).unwrap() = NetworkedActor::Player {
                spawn: player_spawn.clone(),
                status_effects: StatusEffects::default(),
                teleport_query: TeleportQuery::default(),
                distance_range: DistanceRange::Normal,
                conditions: Conditions::default(),
                executing_gimmick_jump: false,
            };
        }
        ToServer::ChangeZone(from_id, actor_id, zone_id, new_position, new_rotation) => {
            tracing::info!("{from_id:?} is requesting to go to zone {zone_id}");

            let mut data = data.lock();
            let mut network = network.lock();

            // create a new instance if necessary
            let mut game_data = game_data.lock();
            data.ensure_exists(*zone_id, &mut game_data);

            // inform the players in this zone that this actor left
            if let Some(current_instance) = data.find_actor_instance_mut(*actor_id) {
                current_instance.actors.remove(actor_id);
                network.inform_remove_actor(current_instance, *from_id, *actor_id);
            }

            // then find or create a new instance with the zone id
            data.ensure_exists(*zone_id, &mut game_data);
            let target_instance = data.find_instance_mut(*zone_id).unwrap();
            target_instance.insert_empty_actor(*actor_id);

            // tell the client to load into the zone
            let msg = FromServer::ChangeZone(
                *zone_id,
                target_instance.content_finder_condition_id,
                target_instance.weather_id,
                new_position.unwrap_or_default(),
                new_rotation.unwrap_or_default(),
                target_instance.zone.to_lua_zone(target_instance.weather_id),
                false,
            );
            network.send_to(*from_id, msg, DestinationNetwork::ZoneClients);
        }
        ToServer::EnterZoneJump(from_id, actor_id, exitbox_id) => {
            let mut data = data.lock();
            let mut network = network.lock();
            let mut game_data = game_data.lock();

            // first, find the zone jump in the current zone
            let destination_zone_id;
            let destination_instance_id;
            if let Some(current_instance) = data.find_actor_instance(*actor_id) {
                let Some((_, new_exit_box)) = current_instance.zone.find_exit_box(*exitbox_id)
                else {
                    tracing::warn!("Couldn't find exit box {exitbox_id}?!");
                    return;
                };
                destination_zone_id = new_exit_box.territory_type;
                destination_instance_id = new_exit_box.destination_instance_id;
            } else {
                tracing::warn!("Actor isn't in the instance it was expected in. This is a bug!");
                return;
            }

            change_zone_warp_to_pop_range(
                &mut data,
                &mut network,
                &mut game_data,
                destination_zone_id,
                destination_instance_id,
                *actor_id,
                *from_id,
            );
        }
        ToServer::Warp(from_id, actor_id, warp_id) => {
            let mut data = data.lock();
            let mut network = network.lock();
            let mut game_data = game_data.lock();

            // first, find the warp and it's destination
            let (destination_instance_id, destination_zone_id) = game_data
                .get_warp(*warp_id)
                .expect("Failed to find the warp!");

            change_zone_warp_to_pop_range(
                &mut data,
                &mut network,
                &mut game_data,
                destination_zone_id,
                destination_instance_id,
                *actor_id,
                *from_id,
            );
        }
        ToServer::WarpAetheryte(from_id, actor_id, aetheryte_id) => {
            let mut data = data.lock();
            let mut network = network.lock();
            let mut game_data = game_data.lock();

            // first, find the warp and it's destination
            let (destination_instance_id, destination_zone_id) = game_data
                .get_aetheryte(*aetheryte_id)
                .expect("Failed to find the aetheryte!");

            change_zone_warp_to_pop_range(
                &mut data,
                &mut network,
                &mut game_data,
                destination_zone_id,
                destination_instance_id,
                *actor_id,
                *from_id,
            );
        }
        ToServer::ZoneIn(from_id, from_actor_id, is_teleport) => {
            tracing::info!("Player {from_id:?} has finally zoned in, informing other players...");

            // Inform all clients to play the zone in animation
            let mut network = network.lock();
            let mut to_remove = Vec::new();
            for (id, (handle, _)) in &mut network.clients {
                let id = *id;

                let category = ActorControlCategory::ZoneIn {
                    warp_finish_anim: 1,
                    raise_anim: 0,
                    unk1: if *is_teleport { 110 } else { 0 },
                };

                if id == *from_id {
                    let msg = FromServer::ActorControlSelf(ActorControlSelf { category });

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                } else {
                    let msg = FromServer::ActorControl(*from_actor_id, ActorControl { category });

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            network.to_remove.append(&mut to_remove);
        }
        ToServer::MoveToPopRange(from_id, from_actor_id, id, fade_out) => {
            let send_new_position = |from_id: ClientId,
                                     network: Arc<Mutex<NetworkState>>,
                                     transform: Transformation,
                                     fade_out: bool| {
                let mut network = network.lock();
                let trans = transform.translation;

                let msg = FromServer::NewPosition(
                    Position {
                        x: trans[0],
                        y: trans[1],
                        z: trans[2],
                    },
                    euler_to_direction(transform.rotation),
                    fade_out,
                );

                // send new position to the client
                network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
            };

            let transform;
            {
                let mut data = data.lock();
                let instance = data.find_actor_instance_mut(*from_actor_id);

                if let Some(instance) = instance {
                    if let Some(pop_range) = instance.zone.find_pop_range(*id) {
                        transform = pop_range.0.transform;
                    } else {
                        tracing::warn!("Failed to find pop range for {id}!");
                        return;
                    }
                } else {
                    return;
                }
            }

            // If fading out, we need to delay the actual warp ever so slightly.
            // Otherwise it actually happens before the screen fades out.
            if *fade_out {
                let network = network.clone();
                let from_id = *from_id;
                tokio::task::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_millis(1000));
                    interval.tick().await;
                    interval.tick().await;
                    send_new_position(from_id, network, transform, true);
                });
            } else {
                send_new_position(*from_id, network, transform, *fade_out);
            }
        }
        _ => {}
    }
}
