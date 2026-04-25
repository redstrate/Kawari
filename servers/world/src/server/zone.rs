use std::{collections::HashMap, sync::Arc};

use glam::{Affine3A, EulerRot, Vec3};
use parking_lot::Mutex;
use physis::{
    TerritoryIntendedUse,
    layer::{
        ExitRangeInstanceObject, InstanceObject, LayerEntryData, PopRangeInstanceObject,
        TriggerBoxShape,
    },
    lgb::Lgb,
    lvb::Lvb,
};

use crate::{
    ClientId, FromServer, GameData, StatusEffects, TerritoryNameKind, ToServer,
    lua::LuaZone,
    server::{
        NetworkedActor, WorldServer,
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
    },
    zone_connection::{BaseParameters, TeleportQuery},
};
use kawari::{
    common::{
        DistanceRange, DropIn, DropInLayer, DropInObjectData, ENTRANCE_CIRCLE_IDS, EOBJ_EXIT,
        EOBJ_HOUSING_ENTRANCE, EOBJ_SHORTCUT, EOBJ_SHORTCUT_EXPLORER_MODE, HandlerType,
        InvisibilityFlags, ObjectId, Position, WARP_DELAY, euler_to_direction,
        internal_housing_row,
    },
    config::get_config,
    ipc::zone::{
        ActorControlCategory, ActorSetPos, BattleNpcSubKind, CharacterDataFlag, CommonSpawn,
        Conditions, DisplayFlag, ObjectKind, ServerZoneIpcData, ServerZoneIpcSegment, SpawnNpc,
        SpawnObject, SpawnTreasure, WarpType,
    },
};

#[derive(Debug)]
pub enum MapGimmick {
    /// Seen for final boss triggers in Sastasha
    Generic {},
    /// Jump pads like the ones in Gold Saucer.
    Jump {
        /// The position to land on.
        to_position: Vec3,
        /// The GimmickJump type.
        gimmick_jump_type: u32,
        /// The animation ID to play for the EObj.
        sgb_animation_id: u32,
        /// The EObj's instance ID to play the animation for.
        eobj_instance_id: u32,
    },
    /// Unsure of what to call these, but these are "exit lines" like as seen in the overworld but go to another poprange in the same zone.
    /// Used heavily in instanced content.
    FakeExit { exit_pop_range_id: u32 },
}

/// Simpler form of a MapRange object designed for collision detection.
#[derive(Debug)]
pub struct MapRange {
    /// Trigger box shape.
    pub trigger_box_shape: TriggerBoxShape,
    /// Position of this range in the world.
    pub position: Vec3,
    /// Relative scale of this range.
    pub scale: Vec3,
    /// Whether this map range represents a sanctuary.
    pub sanctuary: bool,
    /// Whether this map range represents a PvP duel area.
    pub duel: bool,
    /// Whether this map range represents a gimmick, like a jumping pad.
    pub gimmick: Option<MapGimmick>,
    /// Game Object ID. Also known as the layout ID. The client sends this when discovering new areas.
    pub instance_id: u32,
    /// The MapRange's discovery index. Unclear if this is the same as DiscoveryIndex on the Map sheet.
    pub discovery_id: Option<u8>,
    /// Whether this map range represents an instance exit.
    pub entrance: bool,
}

#[derive(Debug)]
struct HousingPlot {
    entrance_position: Vec3,
}

/// Represents a loaded zone
#[derive(Default, Debug)]
pub struct Zone {
    pub id: u16,
    pub internal_name: String,
    pub region_name: String,
    pub place_name: String,
    pub intended_use: u8,
    pub layer_groups: Vec<Lgb>,
    pub navimesh_path: String,
    pub map_id: u16,
    cached_npc_base_ids: HashMap<u32, u32>,
    pub map_ranges: Vec<MapRange>,
    dropin_layers: Vec<DropInLayer>,
    cached_objects: HashMap<u32, SpawnObject>,
    cached_npcs: HashMap<u32, SpawnNpc>,
    cached_treasure: HashMap<u8, SpawnTreasure>,
    layer_set: i32,
    bg_path: String,
    cached_housing_plots: Vec<HousingPlot>,
}

impl Zone {
    pub fn load(game_data: &mut GameData, id: u16) -> Self {
        let mut zone = Self {
            id,
            ..Default::default()
        };

        let Some(row) = game_data.territory_type_sheet.row(id as u32) else {
            tracing::warn!("Invalid zone id {id}, allowing anyway...");
            return zone;
        };

        zone.intended_use = row.TerritoryIntendedUse();
        zone.map_id = row.Map();

        // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
        let bg_path = row.Bg();
        if bg_path.is_empty() {
            tracing::warn!("Invalid zone id {id}, allowing anyway...");
            return zone;
        }

        let path = format!("bg/{}.lvb", &bg_path);
        tracing::info!("Loading {}", path);
        if let Ok(lvb) = game_data.resource.parsed::<Lvb>(&path) {
            let mut load_lgb = |path: &str| -> Option<Lgb> {
                // Skip LGBs that aren't relevant for the server
                if path.ends_with("bg.lgb")
                    || path.ends_with("vfx.lgb")
                    || path.ends_with("sound.lgb")
                {
                    return None;
                }

                let lgb = game_data.resource.parsed::<Lgb>(path);

                tracing::info!("Loading {path}");
                if let Err(e) = &lgb {
                    tracing::warn!(
                        "Failed to parse {path}: {e}, this is most likely a bug in Physis and should be reported somewhere!"
                    )
                }

                lgb.ok()
            };

            for path in &lvb.sections[0].lgb_paths {
                if let Some(lgb) = load_lgb(path) {
                    zone.layer_groups.push(lgb);
                }
            }

            for layer_set in &lvb.sections[0].layer_sets.layer_sets {
                if layer_set.territory_type_id == id {
                    zone.layer_set = layer_set.id;
                    zone.navimesh_path = layer_set
                        .nvm_path
                        .value
                        .replace("/server/data/", "")
                        .to_string();

                    break;
                }
            }

            let mut search_dirs: Vec<String> = get_config()
                .filesystem
                .additional_resource_paths
                .iter()
                .cloned()
                .map(|mut x| {
                    x.push_str("/dropins/");
                    x
                })
                .collect();
            search_dirs.push("resources/dropins/".to_string());

            'outer: for search_dir in search_dirs {
                // Load drop-ins
                for entry in std::fs::read_dir(search_dir)
                    .expect("Didn't find dropins directory?")
                    .flatten()
                {
                    if let Ok(contents) = std::fs::read_to_string(entry.path())
                        && let Ok(mut dropin) = serde_json::from_str::<DropIn>(&contents)
                        && lvb.sections[0].lgb_paths.contains(&dropin.appends)
                    {
                        tracing::info!("Loaded dropin from {:?}", entry.path());
                        zone.dropin_layers.append(&mut dropin.layers);
                        break 'outer;
                    }
                }
            }

            zone.bg_path = lvb.sections[0].general.bg_path.value.clone();
        }

        // create NPC ID cache
        for layer_group in &zone.layer_groups {
            for chunk in &layer_group.chunks {
                for layer in &chunk.layers {
                    if !layer.header.has_layer_set(zone.layer_set as u32) {
                        continue;
                    }

                    for object in &layer.objects {
                        let (scale, _, translation) =
                            Affine3A::from(object.transform).to_scale_rotation_translation();

                        if let LayerEntryData::EventNPC(npc) = &object.data {
                            zone.cached_npc_base_ids
                                .insert(object.instance_id, npc.parent_data.parent_data.base_id);
                        }
                        if let LayerEntryData::MapRange(map_range) = &object.data {
                            zone.map_ranges.push(MapRange {
                                trigger_box_shape: map_range.parent_data.trigger_box_shape,
                                position: translation,
                                scale,
                                sanctuary: map_range.rest_bonus_enabled,
                                duel: false,
                                gimmick: None,
                                instance_id: object.instance_id,
                                discovery_id: if map_range.discovery_enabled {
                                    Some(map_range.discovery_id)
                                } else {
                                    None
                                },
                                entrance: false,
                            });
                        }
                        if let LayerEntryData::EventRange(event_range) = &object.data {
                            zone.map_ranges.push(MapRange {
                                trigger_box_shape: event_range.parent_data.trigger_box_shape,
                                position: translation,
                                scale,
                                sanctuary: false,
                                // This is guesswork since there's only one dueling location in-game
                                duel: event_range.unk_flags[0] == 1
                                    && event_range.unk_flags[3] == 1
                                    && event_range.unk_flags[4] == 1
                                    && event_range.unk_flags[5] == 1,
                                gimmick: None,
                                instance_id: object.instance_id,
                                discovery_id: None,
                                // Set later!
                                entrance: false,
                            });
                        }
                    }

                    // Second pass for eobjs
                    for object in &layer.objects {
                        // if !layer.header.has_layer_set(zone.layer_set as u32) {
                        //     continue;
                        // }

                        if let LayerEntryData::EventObject(eobj) = &object.data {
                            let eobj_data = game_data.get_eobj_data(eobj.parent_data.base_id);
                            let event_type = HandlerType::from_repr(eobj_data >> 16);

                            if let Some(HandlerType::GimmickRect) = event_type {
                                // GimmickRects are used for stuff like the Golden Saucer jumping pads, and is handled server-side.
                                // Thus, we need to go through and mark these MapRanges to play said event.
                                if let Some(gimmick_rect_info) =
                                    game_data.get_gimmick_rect_info(eobj_data & 0xFFFF)
                                    && let Some(target_pop_range) =
                                        zone.find_pop_range(gimmick_rect_info.Params()[1])
                                {
                                    let gimmick_jump_type = gimmick_rect_info.Params()[0];
                                    let target_event_range = gimmick_rect_info.LayoutID();
                                    let sgb_animation_id = gimmick_rect_info.Params()[2];

                                    // 8 seems to indicate a jumping pad
                                    if gimmick_rect_info.TriggerIn() == 8 {
                                        let (_, _, translation) =
                                            Affine3A::from(target_pop_range.0.transform)
                                                .to_scale_rotation_translation();

                                        let map_gimmick = MapGimmick::Jump {
                                            to_position: translation,
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
                                            gimmick_rect_info.TriggerIn()
                                        );
                                    }
                                } else {
                                    tracing::warn!(
                                        "Failed to lookup Gimmick {}?!",
                                        eobj_data & 0xFFFF
                                    );
                                }
                            }
                        } else if let LayerEntryData::EventRange(_) = &object.data
                            && let Some(gimmick_rect_info) =
                                game_data.lookup_gimmick_rect(object.instance_id)
                        {
                            let mut map_gimmick = None;
                            match gimmick_rect_info.TriggerIn() {
                                1 | 18 => {
                                    // FIXME: 1 is seen for cutscene triggers in Sastasha, while 18 is seen for Variant Dungeon routes in A Merchant's Tale. We should make this less "generic".
                                    map_gimmick = Some(MapGimmick::Generic {});
                                }
                                6 => {
                                    // Seen for same-zone "exit ranges" like the one in the beginning of Sycrus Tower
                                    map_gimmick = Some(MapGimmick::FakeExit {
                                        exit_pop_range_id: gimmick_rect_info.Params()[0],
                                    });
                                }
                                _ => tracing::warn!(
                                    "Unknown GimmickRect type: {}",
                                    gimmick_rect_info.TriggerIn()
                                ),
                            }

                            for map_range in &mut zone.map_ranges {
                                if map_range.instance_id == object.instance_id {
                                    map_range.gimmick = map_gimmick;
                                    break;
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

        // create housing plot cache
        if zone.intended_use == TerritoryIntendedUse::HousingOutdoor as u8 {
            let land_sets = game_data
                .get_land_sets(internal_housing_row(id).unwrap())
                .unwrap();
            for land_set in land_sets {
                let map_ranges: Vec<&MapRange> = zone
                    .map_ranges
                    .iter()
                    .filter(|x| x.instance_id == land_set.UnknownRange1) // NOTE: Will be MapRange in the future
                    .collect();
                if map_ranges.is_empty() {
                    tracing::warn!(
                        "Failed to find map range for a plot! The entrance won't spawn!"
                    );
                } else {
                    let map_range = map_ranges.first().unwrap();

                    zone.cached_housing_plots.push(HousingPlot {
                        entrance_position: map_range.position,
                    });
                }
            }
        }

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
                if !layer.header.has_layer_set(self.layer_set as u32) {
                    continue;
                }

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
                if !layer.header.has_layer_set(self.layer_set as u32) {
                    continue;
                }

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

    fn find_entrance_from_base_id(&self, base_id: u32) -> Option<&InstanceObject> {
        // First, we need to find the EventObject for the entrance:
        let mut bound_id = None;
        for layer_group in &self.layer_groups {
            for layer in &layer_group.chunks[0].layers {
                if !layer.header.has_layer_set(self.layer_set as u32) {
                    continue;
                }

                for object in &layer.objects {
                    if let LayerEntryData::EventObject(eobj) = &object.data
                        && eobj.parent_data.base_id == base_id
                    {
                        bound_id = Some(eobj.bound_instance_id);
                        break;
                    }
                }
            }
        }

        bound_id?;

        // Then find the linked instance object, which is usually a SGB.
        for layer_group in &self.layer_groups {
            for layer in &layer_group.chunks[0].layers {
                if !layer.header.has_layer_set(self.layer_set as u32) {
                    continue;
                }

                for object in &layer.objects {
                    if object.instance_id == bound_id.unwrap() {
                        return Some(object);
                    }
                }
            }
        }

        None
    }

    /// Tries to locate the entrance circle used in instanced content.
    pub fn find_entrance(&self) -> Option<&InstanceObject> {
        for base_id in ENTRANCE_CIRCLE_IDS {
            if let Some(object) = self.find_entrance_from_base_id(base_id) {
                return Some(object);
            }
        }

        None
    }

    /// Returns a list of event objects to spawn by default. If `explorer_mode`, replaces the shortcut object.
    ///
    /// For example, the Gold Saucer arcade machines or shortcuts in dungeons.
    pub fn get_event_objects(
        &mut self,
        game_data: &mut GameData,
        explorer_mode: bool,
    ) -> Vec<SpawnObject> {
        let mut object_spawns = Vec::new();

        for layer_group in &self.layer_groups {
            for layer in &layer_group.chunks[0].layers {
                if !layer.header.has_layer_set(self.layer_set as u32) {
                    continue;
                }

                for object in &layer.objects {
                    let (_, rotation, translation) =
                        Affine3A::from(object.transform).to_scale_rotation_translation();

                    if let LayerEntryData::EventObject(eobj) = &object.data {
                        let unselectable = if let Some(event_type) = HandlerType::from_repr(
                            game_data.get_eobj_data(eobj.parent_data.base_id) >> 16,
                        ) {
                            matches!(event_type, HandlerType::Invalid | HandlerType::GimmickRect)
                        } else {
                            true // make it unselectable to be on the safe side.
                        };

                        let base_id = if eobj.parent_data.base_id == EOBJ_SHORTCUT && explorer_mode
                        {
                            EOBJ_SHORTCUT_EXPLORER_MODE
                        } else {
                            eobj.parent_data.base_id
                        };

                        // Hide shortcuts and exits, these will be spawned by the director.
                        let visibility = if eobj.parent_data.base_id == EOBJ_SHORTCUT
                            || eobj.parent_data.base_id == EOBJ_EXIT
                        {
                            InvisibilityFlags::UNK1
                                | InvisibilityFlags::UNK2
                                | InvisibilityFlags::UNK3
                        } else {
                            InvisibilityFlags::VISIBLE
                        };

                        let spawn = SpawnObject {
                            kind: ObjectKind::EventObj,
                            base_id,
                            unselectable,
                            visibility,
                            entity_id: ObjectId(fastrand::u32(..)),
                            layout_id: object.instance_id,
                            bind_layout_id: eobj.bound_instance_id,
                            radius: 1.0,
                            rotation: euler_to_direction(rotation.to_euler(EulerRot::XYZ)),
                            position: Position(translation),
                            ..Default::default()
                        };
                        self.cached_objects.insert(eobj.parent_data.base_id, spawn);

                        if game_data.get_eobj_pop_type(eobj.parent_data.base_id) == 1 {
                            object_spawns.push(spawn);
                        }
                    }

                    if let LayerEntryData::Treasure(treasure) = &object.data {
                        self.cached_treasure.insert(
                            treasure.base_id,
                            SpawnTreasure {
                                base_id: treasure.base_id as u32,
                                entity_id: ObjectId(fastrand::u32(..)),
                                layout_id: object.instance_id,
                                rotation: euler_to_direction(rotation.to_euler(EulerRot::XYZ)),
                                position: Position(translation),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }

        // Only dropins are checked for gathering points, because they strip that from retail LGBs.
        for layer in &self.dropin_layers {
            for object in &layer.objects {
                if let DropInObjectData::GatheringPoint { base_id } = object.data {
                    let spawn = SpawnObject {
                        kind: ObjectKind::GatheringPoint,
                        base_id,
                        entity_id: ObjectId(fastrand::u32(..)),
                        layout_id: object.instance_id,
                        radius: 1.0,
                        args1: 50334724, // TODO: what is this value? it varies between nodes, and I *believe* it has to be about grouping.
                        position: object.position,
                        ..Default::default()
                    };
                    self.cached_objects.insert(base_id, spawn);
                    object_spawns.push(spawn);
                }
            }
        }

        // housing plot entrances
        for (i, plot) in self.cached_housing_plots.iter().enumerate() {
            let spawn = SpawnObject {
                kind: ObjectKind::EventObj,
                base_id: EOBJ_HOUSING_ENTRANCE,
                entity_id: ObjectId(fastrand::u32(..)),
                radius: 1.0,
                position: Position(plot.entrance_position),
                args2: u32::from_le_bytes([0, i as u8, 0, 0]),
                ..Default::default()
            };
            object_spawns.push(spawn);
        }

        object_spawns
    }

    /// Returns an SpawnObject for the given base ID.
    pub fn get_event_object(&self, base_id: u32) -> Option<SpawnObject> {
        self.cached_objects.get(&base_id).cloned()
    }

    /// Returns an SpawnNpc for the given instance ID.
    pub fn get_battle_npc(&self, instance_id: u32) -> Option<SpawnNpc> {
        self.cached_npcs.get(&instance_id).cloned()
    }

    /// Returns an SpawnTreasure for the given base ID.
    pub fn get_treasure(&self, base_id: u8) -> Option<SpawnTreasure> {
        self.cached_treasure.get(&base_id).cloned()
    }

    /// Returns a list of battle NPCs to spawn.
    pub fn get_npcs(&mut self, game_data: &mut GameData) -> Vec<SpawnNpc> {
        let mut npc_spawns = Vec::new();

        // Only dropins are checked for battle npcs, because they strip that from retail LGBs.
        for layer in &self.dropin_layers {
            for object in &layer.objects {
                if let DropInObjectData::BattleNpc {
                    base_id,
                    name_id,
                    hp,
                    level,
                    nonpop,
                    hostile,
                    gimmick_id,
                    max_links,
                    link_family,
                    link_range,
                } = object.data
                {
                    let (model_chara, battalion, customize, rank, equip) =
                        game_data.find_bnpc(base_id).unwrap();

                    let usable_hp;
                    if let Some(hp) = hp {
                        usable_hp = hp;
                    } else {
                        let modifiers = game_data
                            .get_class_job_modifiers(0)
                            .expect("Failed to read param grow");

                        let attributes = game_data
                            .get_racial_base_attributes(0)
                            .expect("Failed to read racial attributes");

                        let param_grow = game_data
                            .get_param_grow(level)
                            .expect("Failed to read param grow");

                        let mut base_parameters = BaseParameters::default();
                        base_parameters.calculate_based_on_level(
                            &attributes,
                            level,
                            &param_grow,
                            &modifiers,
                        );
                        base_parameters.calculate_potencies(level, &param_grow);

                        usable_hp = base_parameters.hp;
                    }

                    let spawn = SpawnNpc {
                        gimmick_id,
                        character_data_flags: if hostile {
                            CharacterDataFlag::HOSTILE
                        } else {
                            CharacterDataFlag::NONE
                        },
                        character_data_icon: rank,
                        max_links,
                        link_family,
                        link_range,
                        common: CommonSpawn {
                            base_id,
                            name_id,
                            max_health_points: usable_hp,
                            health_points: usable_hp,
                            model_chara,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            battalion,
                            level: level as u8,
                            position: object.position,
                            rotation: object.rotation,
                            look: customize,
                            layout_id: object.instance_id,
                            ..game_data.get_npc_equip(equip as u32).unwrap_or_default()
                        },
                        ..Default::default()
                    };

                    self.cached_npcs.insert(object.instance_id, spawn.clone());
                    if !nonpop {
                        npc_spawns.push(spawn);
                    }
                }
                if let DropInObjectData::EventNpc { base_id } = object.data {
                    let (model_chara, customize, equip) = game_data.find_enpc(base_id).unwrap();

                    let spawn = SpawnNpc {
                        common: CommonSpawn {
                            base_id,
                            name_id: base_id,
                            model_chara,
                            object_kind: ObjectKind::EventNpc,
                            position: object.position,
                            rotation: object.rotation,
                            look: customize,
                            layout_id: object.instance_id,
                            ..game_data.get_npc_equip(equip as u32).unwrap_or_default()
                        },
                        ..Default::default()
                    };

                    self.cached_npcs.insert(object.instance_id, spawn.clone());
                    npc_spawns.push(spawn);
                }
            }
        }

        npc_spawns
    }

    /// Returns a list of MapRanges that overlap this position.
    pub fn get_overlapping_map_ranges(&self, position: Vec3) -> Vec<&MapRange> {
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

                    let pt1 = Vec3 {
                        x: map_range.position.x,
                        y: map_range.position.y - map_range.scale[1],
                        z: map_range.position.z,
                    };
                    let pt2 = Vec3 {
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
                _ => {} // TODO: support other box shapes
            }
        }

        overlapping
    }

    // From https://www.flipcode.com/archives/Fast_Point-In-Cylinder_Test.shtml
    fn cylinder_test(pt1: Vec3, pt2: Vec3, length_sq: f32, radius_sq: f32, test_pt: Vec3) -> f32 {
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
    destination_zone_id: Option<u16>,
    actor_id: ObjectId,
    warp_type: WarpType,
    param4: u8,
    hide_character: u8,
    unk1: u8,
) -> (&'a mut Instance, bool) {
    if let Some(destination_zone_id) = destination_zone_id {
        let mut needs_init_zone = false;

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PrepareZoning {
            target_zone: destination_zone_id,
            warp_type,
            fade_out_time: 1,
            log_message: 0,
            animation: 0,
            param4,
            hide_character,
            param_7: 0,
            unk1,
            unk2: 0,
        });

        network.send_to_by_actor_id(
            actor_id,
            FromServer::PacketSegment(ipc, actor_id),
            DestinationNetwork::ZoneClients,
        );

        // inform the players in this zone that this actor left
        if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
            // HACK: This is to prevent actors from disappearing when warping within the same zone.
            if current_instance.zone.id != destination_zone_id {
                network.remove_actor(current_instance, actor_id);
                needs_init_zone = true;
            }
        }

        // then find or create a new instance with the zone id
        let instance = data.ensure_exists(destination_zone_id, game_data);
        // Insert an empty actor that will be filled later
        instance.insert_empty_actor(actor_id);

        (instance, needs_init_zone)
    } else {
        let instance = data.find_actor_instance_mut(actor_id).unwrap();

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PrepareZoning {
            target_zone: instance.zone.id,
            warp_type,
            fade_out_time: 1,
            log_message: 0,
            animation: 0,
            param4,
            hide_character,
            param_7: 0,
            unk1,
            unk2: 0,
        });

        network.send_to_by_actor_id(
            actor_id,
            FromServer::PacketSegment(ipc, actor_id),
            DestinationNetwork::ZoneClients,
        );

        (instance, false)
    }
}

/// Sends the needed information to ZoneConnection for a zone change.
pub fn change_zone_warp_to_pop_range(
    data: &mut WorldServer,
    network: &mut NetworkState,
    game_data: &mut GameData,
    destination_zone_id: Option<u16>,
    destination_instance_id: u32,
    actor_id: ObjectId,
    from_id: ClientId,
    warp_type: WarpType,
    param4: u8,
    hide_character: u8,
    unk1: u8,
) {
    let (target_instance, needs_init_zone) = begin_change_zone(
        data,
        network,
        game_data,
        destination_zone_id,
        actor_id,
        warp_type,
        param4,
        hide_character,
        unk1,
    );

    let exit_position;
    let exit_rotation;
    if let Some((destination_object, _)) =
        target_instance.zone.find_pop_range(destination_instance_id)
    {
        let (_, rotation, translation) =
            Affine3A::from(destination_object.transform).to_scale_rotation_translation();
        exit_position = Some(Position(translation));
        exit_rotation = Some(euler_to_direction(rotation.to_euler(EulerRot::XYZ)));
    } else {
        exit_position = None;
        exit_rotation = None;
    }

    do_change_zone(
        network,
        target_instance,
        needs_init_zone,
        exit_position,
        exit_rotation,
        from_id,
        warp_type,
    );
}

/// Sends the needed information to ZoneConnection for a zone change.
pub fn change_zone_warp_to_entrance(
    network: &mut NetworkState,
    target_instance: &mut Instance,
    needs_init_zone: bool,
    from_id: ClientId,
) {
    let exit_position;
    let exit_rotation;
    if let Some(destination_object) = target_instance.zone.find_entrance() {
        let (_, rotation, translation) =
            Affine3A::from(destination_object.transform).to_scale_rotation_translation();
        exit_position = Some(Position(translation));
        exit_rotation = Some(euler_to_direction(rotation.to_euler(EulerRot::XYZ)));
    } else {
        tracing::warn!(
            "Failed to find instanced content entrance?! This is a bug in Kawari, please report it!"
        );
        exit_position = None;
        exit_rotation = None;
    }

    do_change_zone(
        network,
        target_instance,
        needs_init_zone,
        exit_position,
        exit_rotation,
        from_id,
        WarpType::Normal,
    );
}

/// Teleports one player to another.
pub fn change_zone_to_player(
    network: &mut NetworkState,
    data: &mut WorldServer,
    game_data: &mut GameData,
    from_id: ClientId,
    to_actor_id: ObjectId,
) {
    let destination_zone_id;
    {
        let Some(target_instance) = data.find_actor_instance(to_actor_id) else {
            return;
        };

        destination_zone_id = target_instance.zone.id;
    }

    let from_actor_id = network.clients.get(&from_id).unwrap().0.actor_id;

    let (target_instance, needs_init_zone) = begin_change_zone(
        data,
        network,
        game_data,
        Some(destination_zone_id),
        from_actor_id,
        WarpType::Normal,
        0,
        0,
        0,
    );

    let Some(target_actor) = target_instance.find_actor(to_actor_id) else {
        return;
    };

    do_change_zone(
        network,
        target_instance,
        needs_init_zone,
        Some(target_actor.position()),
        Some(target_actor.rotation()),
        from_id,
        WarpType::Normal,
    );
}

/// Sends the needed information to ZoneConnection for a zone change.
fn do_change_zone(
    network: &mut NetworkState,
    target_instance: &mut Instance,
    needs_init_zone: bool,
    exit_position: Option<Position>,
    exit_rotation: Option<f32>,
    from_id: ClientId,
    warp_type: WarpType,
) {
    let actor_id = network.clients.get(&from_id).unwrap().0.actor_id;
    let state = network.get_state_mut(from_id).unwrap();

    if needs_init_zone {
        // Clear spawn pools
        state.actor_allocator.clear();
        state.object_allocator.clear();

        let director_vars = target_instance
            .director
            .as_ref()
            .map(|director| director.build_var_segment());

        // now that we have all of the data needed, inform the connection of where they need to be
        let msg = FromServer::ChangeZone(
            target_instance.zone.id,
            target_instance.content_finder_condition_id,
            target_instance.weather_id,
            exit_position.unwrap_or_default(),
            exit_rotation.unwrap_or_default(),
            target_instance.zone.to_lua_zone(target_instance.weather_id),
            false,
            director_vars,
        );
        network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
    } else {
        // We want to delay sending this to give time for the client to fade out.
        let segment = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorSetPos(ActorSetPos {
            position: exit_position.unwrap_or_default(),
            rotation: exit_rotation.unwrap_or_default(),
            warp_type,
            warp_type_arg: 2, // unknown
            ..Default::default()
        }));
        target_instance.insert_task(
            from_id,
            actor_id,
            WARP_DELAY,
            QueuedTaskData::PacketSegment { segment },
        );
    }
}

/// Process zone-related messages.
pub fn handle_zone_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    game_data: Arc<Mutex<GameData>>,
    msg: &ToServer,
) -> bool {
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
                inside_instance_exit: false,
                parameters: BaseParameters::default(),
                dueling_opponent_id: ObjectId::default(),
                remove_cooldowns: false,
            };

            true
        }
        ToServer::ChangeZone(
            from_id,
            actor_id,
            zone_id,
            new_position,
            new_rotation,
            warp_type_info,
        ) => {
            tracing::info!("{from_id:?} is requesting to go to zone {zone_id}");

            let mut data = data.lock();
            let mut network = network.lock();
            let mut game_data = game_data.lock();

            let (warp_type, param4, hide_character, unk1) =
                if let Some((w_type, param, hide, unk)) = warp_type_info {
                    (*w_type, *param, *hide, *unk)
                } else {
                    (WarpType::Normal, 0, 0, 0)
                };

            let (target_instance, needs_init_zone) = begin_change_zone(
                &mut data,
                &mut network,
                &mut game_data,
                Some(*zone_id),
                *actor_id,
                warp_type,
                param4,
                hide_character,
                unk1,
            );
            do_change_zone(
                &mut network,
                target_instance,
                needs_init_zone,
                *new_position,
                *new_rotation,
                *from_id,
                warp_type,
            );

            true
        }
        ToServer::EnterZoneJump(from_id, actor_id, exitbox_id, warp_type_info) => {
            let mut data = data.lock();
            let mut network = network.lock();

            // first, find the zone jump in the current zone
            let mut destination_zone_id;
            let destination_instance_id;
            if let Some(current_instance) = data.find_actor_instance(*actor_id) {
                let Some((_, new_exit_box)) = current_instance.zone.find_exit_box(*exitbox_id)
                else {
                    tracing::warn!("Couldn't find exit box {exitbox_id}?!");
                    return true;
                };
                destination_zone_id = new_exit_box.territory_type;

                // Seen when attempting to enter underwater portals in Ruby Sea
                if new_exit_box.territory_type == 0
                    && new_exit_box.zone_id == 0
                    && new_exit_box.exit_type == physis::layer::ExitType::Unk
                {
                    destination_zone_id = current_instance.zone.id;
                }

                destination_instance_id = new_exit_box.destination_instance_id;
            } else {
                tracing::warn!("Actor isn't in the instance it was expected in. This is a bug!");
                return true;
            }

            let (warp_type, param4, hide_character, unk1) =
                if let Some((w_type, param, hide, unk)) = warp_type_info {
                    (*w_type, *param, *hide, *unk)
                } else {
                    (WarpType::Normal, 0, 0, 0)
                };

            let mut game_data = game_data.lock();
            change_zone_warp_to_pop_range(
                &mut data,
                &mut network,
                &mut game_data,
                Some(destination_zone_id),
                destination_instance_id,
                *actor_id,
                *from_id,
                warp_type,
                param4,
                hide_character,
                unk1,
            );

            true
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
                Some(destination_zone_id),
                destination_instance_id,
                *actor_id,
                *from_id,
                WarpType::Normal,
                0,
                0,
                0,
            );

            true
        }
        ToServer::WarpAetheryte(from_id, actor_id, aetheryte_id, housing_aethernet) => {
            let mut data = data.lock();
            let mut network = network.lock();
            let mut game_data = game_data.lock();

            // first, find the warp and it's destination
            let (destination_instance_id, destination_zone_id) = game_data
                .get_aetheryte(*aetheryte_id, *housing_aethernet)
                .expect("Failed to find the aetheryte!");

            change_zone_warp_to_pop_range(
                &mut data,
                &mut network,
                &mut game_data,
                Some(destination_zone_id),
                destination_instance_id,
                *actor_id,
                *from_id,
                WarpType::Normal,
                0,
                0,
                0,
            );

            true
        }
        ToServer::WarpPopRange(from_id, from_actor_id, territory_id, pop_range_id) => {
            let mut data = data.lock();
            let mut network = network.lock();
            let mut game_data = game_data.lock();

            change_zone_warp_to_pop_range(
                &mut data,
                &mut network,
                &mut game_data,
                Some(*territory_id),
                *pop_range_id,
                *from_actor_id,
                *from_id,
                WarpType::Normal,
                0,
                0,
                0,
            );

            true
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
                    let msg = FromServer::ActorControlSelf(category);

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                } else {
                    let msg = FromServer::ActorControl(*from_actor_id, category);

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            network.to_remove.append(&mut to_remove);

            // Then update the PlayerSpawn so respawning this player doesn't appear invisible again
            let mut data = data.lock();
            if let Some(instance) = data.find_actor_instance_mut(*from_actor_id)
                && let Some(actor) = instance.find_actor_mut(*from_actor_id)
            {
                actor
                    .get_common_spawn_mut()
                    .display_flags
                    .remove(DisplayFlag::INVISIBLE);
            }

            true
        }
        ToServer::MoveToPopRange(from_id, from_actor_id, id, fade_out) => {
            let zone_id;
            {
                let data = data.lock();
                let Some(instance) = data.find_actor_instance(*from_actor_id) else {
                    return false;
                };

                zone_id = instance.zone.id;
            }

            let mut data = data.lock();
            let mut network = network.lock();
            let mut game_data = game_data.lock();
            change_zone_warp_to_pop_range(
                &mut data,
                &mut network,
                &mut game_data,
                Some(zone_id),
                *id,
                *from_actor_id,
                *from_id,
                if *fade_out {
                    WarpType::Normal
                } else {
                    WarpType::None
                },
                0,
                0,
                0,
            );

            true
        }
        ToServer::NewLocationDiscovered(from_id, layout_id, _pos, zone_id) => {
            let data = data.lock();
            let mut network = network.lock();

            for instance in &data.instances {
                if instance.zone.id == *zone_id {
                    for range in &instance.zone.map_ranges {
                        if range.instance_id == *layout_id
                            && let Some(discovery_id) = range.discovery_id
                        {
                            // TODO: Check if the player is actually in this range?
                            // TODO: This is the "old" style of map discovery where every chunk is revealed one by one as the player runs into them. It's currently unclear how retail reveals the entire map at once. As an example, for North Shroud, retail sends map_part_id 164, which reveals its entire map. When we enter North Shroud from Old Gridania, Kawari currently sends 1.
                            let mut game_data = game_data.lock();
                            let Some(map_id) = game_data.get_territory_info_map_data(*zone_id)
                            else {
                                tracing::error!(
                                    "Unable to get Map column data from TerritoryInfo sheet for zone id {zone_id}"
                                );
                                return true;
                            };

                            let msg =
                                FromServer::LocationDiscovered(map_id.into(), discovery_id.into());
                            network.send_to(*from_id, msg, DestinationNetwork::ZoneClients);
                            return true;
                        }
                    }

                    // If we somehow didn't get any discoverable ranges, exit early. Is that even possible?
                    break;
                }
            }

            true
        }
        ToServer::PlaceFurniture(
            from_actor_id,
            container,
            slot,
            catalog_id,
            stain,
            position,
            indoors,
            rotation,
            plot_index,
        ) => {
            let mut network = network.lock();
            let data = data.lock();

            let Some(instance) = data.find_actor_instance(*from_actor_id) else {
                return true;
            };

            let msg = FromServer::FurniturePlaced(
                *container,
                *slot,
                *catalog_id,
                *stain,
                *position,
                *indoors,
                *rotation,
                *plot_index,
            );

            // We *do* want to include the sender here
            network.send_in_range_inclusive_instance(
                *from_actor_id,
                instance,
                msg,
                DestinationNetwork::ZoneClients,
            );

            true
        }
        ToServer::TranslateFurniture(
            from_actor_id,
            plot_info,
            slot,
            position,
            rotation,
            indoors,
        ) => {
            let mut network = network.lock();
            let data = data.lock();

            let Some(instance) = data.find_actor_instance(*from_actor_id) else {
                return true;
            };

            let msg =
                FromServer::FurnitureTranslated(*plot_info, *slot, *position, *rotation, *indoors);

            // We *don't* want to include the sender here
            network.send_in_range_instance(
                *from_actor_id,
                instance,
                msg,
                DestinationNetwork::ZoneClients,
            );

            true
        }
        _ => false,
    }
}
