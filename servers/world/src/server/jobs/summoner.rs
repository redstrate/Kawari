//! Summoner (SMN) job-specific logic: action remaps, gauge state, pet lifecycle, and status syncing.

use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use binrw::BinRead;
use glam::Vec3;
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer,
    common::PetCommand,
    gamedata::GameData,
    lua::GaugeAction,
    server::{
        actor::{NetworkedActor, NpcState},
        combat_state::{
            PlayerCombatState, SummonerAttunement, SummonerDemiPhase, SummonerNextDemi,
            SummonerState,
        },
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
        set_character_mode,
    },
};
use kawari::{
    common::{
        CharacterMode, JumpState, MoveAnimationState, MoveAnimationType, ObjectId, ObjectTypeId,
        ObjectTypeKind, Position,
    },
    ipc::zone::{
        ActionEffect, ActionRequest, ActionResult, ActionType, ActorControlCategory,
        BattleNpcSubKind, CharacterDataFlag, DamageElement, DamageType, DisplayFlag, EffectKind,
        ObjectKind, ServerZoneIpcData, ServerZoneIpcSegment, SpawnNpc, SpawnObject, StatusEffect,
        StatusEffectList,
    },
};

/// ClassJob row id for Summoner (SMN). NOTE: 26 is Arcanist (ACN); Summoner is 27. The player's
/// `common.class_job` carries the real ClassJob id (`current_class`), so this must be 27 or every
/// SMN-specific path (gauge, action remaps, gauge state) is silently skipped.
const CLASSJOB_SUMMONER: u8 = 27;

const ACTION_RUIN: u32 = 163;
const ACTION_RUIN_III: u32 = 3579;
const ACTION_RUIN_IV: u32 = 7426;
const ACTION_FESTER: u32 = 181;
const ACTION_PAINFLARE: u32 = 3578;
const ACTION_NECROTIZE: u32 = 36990;
const ACTION_OUTBURST: u32 = 16511;
// Outburst (16511) morphs into Tri-disaster (25826) once the SMN learns Outburst Mastery
// (trait 477, Lv74). Below that level / on Arcanist it stays Outburst.
const ACTION_TRI_DISASTER: u32 = 25826;
const TRAIT_OUTBURST_MASTERY: u32 = 477;
const ACTION_SUMMON_CARBUNCLE: u32 = 25798;
const ACTION_AETHERCHARGE: u32 = 25800;
const ACTION_SEARING_LIGHT: u32 = 25801;
const ACTION_SUMMON_BAHAMUT: u32 = 7427;
const ACTION_SUMMON_SOLAR_BAHAMUT: u32 = 36992;
const ACTION_SUMMON_RUBY: u32 = 25802;
const ACTION_SUMMON_TOPAZ: u32 = 25803;
const ACTION_SUMMON_EMERALD: u32 = 25804;
// Egi-II summons (L90+): the modern replacements for the Ruby/Topaz/Emerald summons. They behave
// identically (grant the matching attunement + ready proc) but also deal damage. Ruby=Ifrit-II,
// Topaz=Titan-II, Emerald=Garuda-II.
const ACTION_SUMMON_IFRIT_II: u32 = 25838;
const ACTION_SUMMON_TITAN_II: u32 = 25839;
const ACTION_SUMMON_GARUDA_II: u32 = 25840;
const ACTION_ASTRAL_FLOW: u32 = 25822;
const ACTION_ASTRAL_IMPULSE: u32 = 25820;
const ACTION_ASTRAL_FLARE: u32 = 25821;
const ACTION_UMBRAL_IMPULSE: u32 = 36994;
const ACTION_UMBRAL_FLARE: u32 = 36995;
const ACTION_GEMSHINE: u32 = 25883;
const ACTION_PRECIOUS_BRILLIANCE: u32 = 25884;
const ACTION_RUBY_RUIN: u32 = 25823;
const ACTION_TOPAZ_RUIN: u32 = 25824;
const ACTION_EMERALD_RUIN: u32 = 25825;
const ACTION_RUBY_DISASTER: u32 = 25832;
const ACTION_TOPAZ_DISASTER: u32 = 25833;
const ACTION_EMERALD_DISASTER: u32 = 25834;
const ACTION_CRIMSON_CYCLONE: u32 = 25835;
const ACTION_MOUNTAIN_BUSTER: u32 = 25836;
const ACTION_SLIPSTREAM: u32 = 25837;
const ACTION_ENERGY_DRAIN: u32 = 16508;
const ACTION_ENERGY_SIPHON: u32 = 16510;
const ACTION_CRIMSON_STRIKE: u32 = 25885;
const ACTION_DEATHFLARE: u32 = 3582;
const ACTION_SEARING_FLASH: u32 = 36991;
const ACTION_SUNFLARE: u32 = 36996;
const ACTION_LUX_SOLARIS: u32 = 36997;
const ACTION_ENKINDLE_BAHAMUT: u32 = 7429;
const ACTION_ENKINDLE_SOLAR_BAHAMUT: u32 = 36998;
const LEVEL_SUMMON_SOLAR_BAHAMUT: u8 = 100;
const ACTION_WYRM_WAVE: u32 = 7428;
const ACTION_SCARLET_FLAME: u32 = 36993;

const STATUS_FURTHER_RUIN: u16 = 2701;
const STATUS_SEARING_LIGHT: u16 = 2703;
const STATUS_CRIMSON_CYCLONE_READY: u16 = 2724;
const STATUS_SLIPSTREAM_READY: u16 = 2725;
const STATUS_MOUNTAIN_BUSTER_READY: u16 = 2853;
const STATUS_GARUDA_ATTUNEMENT: u16 = 3009;
const STATUS_TITAN_ATTUNEMENT: u16 = 3010;
const STATUS_IFRIT_ATTUNEMENT: u16 = 3011;
const STATUS_DREADWYRM_TRANCE: u16 = 3228;
const STATUS_SEARING_FLASH_READY: u16 = 3873;
const STATUS_LUX_SOLARIS_READY: u16 = 3874;
const STATUS_CRIMSON_STRIKE_READY: u16 = 4403;

const SUMMONER_GAUGE_CARBUNCLE: u8 = 23;
const SUMMONER_GAUGE_ATTUNEMENT_RUBY: u8 = 1;
const SUMMONER_GAUGE_ATTUNEMENT_TOPAZ: u8 = 2;
const SUMMONER_GAUGE_ATTUNEMENT_EMERALD: u8 = 3;
const SUMMONER_GAUGE_FLAG_AETHERFLOW_1: u8 = 1 << 0;
const SUMMONER_GAUGE_FLAG_AETHERFLOW_2: u8 = 1 << 1;
const SUMMONER_GAUGE_FLAG_SOLAR_BAHAMUT_FIRST_PRIMED: u8 = 1 << 3;
const SUMMONER_GAUGE_FLAG_SOLAR_BAHAMUT_SECOND_PRIMED: u8 =
    (1 << 2) | SUMMONER_GAUGE_FLAG_SOLAR_BAHAMUT_FIRST_PRIMED;
const SUMMONER_GAUGE_FLAG_IFRIT_READY: u8 = 1 << 5;
const SUMMONER_GAUGE_FLAG_TITAN_READY: u8 = 1 << 6;
const SUMMONER_GAUGE_FLAG_GARUDA_READY: u8 = 1 << 7;
const SUMMONER_ATTUNEMENT_DURATION: Duration = Duration::from_secs(30);
const SUMMONER_DEMI_DURATION: Duration = Duration::from_secs(15);
const SUMMONER_SEARING_LIGHT_DURATION: Duration = Duration::from_secs(20);
const SUMMONER_FURTHER_RUIN_DURATION: Duration = Duration::from_secs(60);
const SUMMONER_READY_STATUS_DURATION: Duration = Duration::from_secs(30);
const SUMMONER_DEMI_AUTO_ATTACKS: u8 = 4;
const SUMMONER_DEMI_AUTO_ATTACK_INTERVAL: Duration = Duration::from_secs(3);
// From ActionTransient.Description:
// 7428 真龙波: 威力 150; 36993 光芒: 威力 160.
const SUMMONER_DEMI_BAHAMUT_AUTO_ATTACK_POTENCY: u32 = 150;
const SUMMONER_DEMI_SOLAR_BAHAMUT_AUTO_ATTACK_POTENCY: u32 = 160;
const SUMMONER_DEMI_BAHAMUT_BASE_ID: u32 = 6982;
const SUMMONER_DEMI_SOLAR_BAHAMUT_BASE_ID: u32 = 16926;
const ACTION_INFERNO: u32 = 25852;
const ACTION_EARTHEN_FURY: u32 = 25853;
const ACTION_AERIAL_BLAST: u32 = 25854;
const SUMMONER_PET_SPAWN_DISTANCE: f32 = 1.75;
const PET_DISMISS_FADE_OUT: Duration = Duration::from_millis(1200);
const PET_TRANSITION_FADE_OUT: Duration = Duration::from_secs(2);
const PET_REVEAL_DELAY: Duration = Duration::from_millis(500);
/// Invalid object id retail uses in `SetupPet.pet_actor_id` while clearing the current pet binding.
const SUMMONER_INVALID_PET_ACTOR_ID: ObjectId = ObjectId(0xE0000000);
/// `SetPetParameters.pet_id` for the Solar Bahamut pet hotbar.
const SUMMONER_PET_HOTBAR_SOLAR_BAHAMUT: u32 = 46;
/// `SetPetParameters.pet_id` for the Bahamut pet hotbar.
const SUMMONER_PET_HOTBAR_BAHAMUT: u32 = 10;
/// `SetPetParameters.pet_id` for the carbuncle pet hotbar.
const SUMMONER_PET_HOTBAR_CARBUNCLE: u32 = 23;
const SUMMONER_PET_HOTBAR_IFRIT: u32 = 30;
const SUMMONER_PET_HOTBAR_TITAN: u32 = 31;
const SUMMONER_PET_HOTBAR_GARUDA: u32 = 32;
const SUMMONER_PRIMAL_FINISHER_POTENCY: u32 = 800;
const SUMMONER_PRIMAL_FINISHER_ANIMATION_LOCK: f32 = 4.10;
const SUMMONER_PRIMAL_FINISHER_DELAY: Duration = Duration::from_millis(2100);
const SUMMONER_PRIMAL_FINISHER_RETRY_DELAY: Duration = Duration::from_millis(300);
const SUMMONER_PRIMAL_WAIT_FOR_TARGET_DURATION: Duration = Duration::from_secs(8);
const DEMI_AUTO_ATTACK_ANIMATION_LOCK: f32 = 1.6;
// ActionTransient[25837]: lingering Slipstream ground AoE potency 30, duration 15s.
const SUMMONER_SLIPSTREAM_GROUND_POTENCY: u32 = 30;
const SUMMONER_SLIPSTREAM_GROUND_RADIUS: f32 = 5.0;
const SUMMONER_SLIPSTREAM_GROUND_TICK_INTERVAL: Duration = Duration::from_secs(3);
const SUMMONER_SLIPSTREAM_GROUND_TICKS: u8 = 5;
const SUMMONER_SLIPSTREAM_GROUND_VFX_DELAY: Duration = Duration::from_millis(1400);
const SUMMONER_SLIPSTREAM_GROUND_VFX_DURATION: Duration = Duration::from_secs(15);
const SUMMONER_SLIPSTREAM_GROUND_VFX_ID: u32 = 691;
// Retail sends Slipstream's lingering damage as ActorControl TickDamage sourced from the target.
// The first field carries the Slipstream Status EXD row id.
const SUMMONER_SLIPSTREAM_TICK_STATUS_ID: u32 = 0x0A92;
const SUMMONER_SLIPSTREAM_TICK_UNK2: u32 = 5;

/// Gauge-resource indices for `EffectsBuilder:modify_gauge(index, amount)` (mirror of Global.lua).
const GAUGE_INDEX_AETHERFLOW: u8 = 0;
/// Maximum Aetherflow stacks.
const MAX_AETHERFLOW: u8 = 2;

#[derive(Debug, Clone, Copy)]
struct SummonerPetSpawnSpec {
    pet_id: u32,
    base_id: u32,
    name_id: u32,
    name: &'static str,
    model_chara: u16,
    display_flags: u32,
    parameter_unk3: u32,
    parameter_unk4: u32,
}

const SUMMONER_CARBUNCLE_SPAWN: SummonerPetSpawnSpec = SummonerPetSpawnSpec {
    pet_id: SUMMONER_PET_HOTBAR_CARBUNCLE,
    base_id: 13498,
    name_id: 10261,
    name: "宝石兽",
    model_chara: 411,
    display_flags: 0x0004_0028,
    parameter_unk3: 5,
    parameter_unk4: 7,
};

const SUMMONER_SOLAR_BAHAMUT_SPAWN: SummonerPetSpawnSpec = SummonerPetSpawnSpec {
    pet_id: SUMMONER_PET_HOTBAR_SOLAR_BAHAMUT,
    base_id: 16926,
    name_id: 13159,
    name: "烈日巴哈姆特",
    model_chara: 4038,
    display_flags: 0x0004_002B,
    parameter_unk3: 0,
    parameter_unk4: 0,
};

const SUMMONER_BAHAMUT_SPAWN: SummonerPetSpawnSpec = SummonerPetSpawnSpec {
    pet_id: SUMMONER_PET_HOTBAR_BAHAMUT,
    base_id: 6982,
    name_id: 6566,
    name: "亚灵神巴哈姆特",
    model_chara: 1930,
    display_flags: 0x0004_002B,
    parameter_unk3: 0,
    parameter_unk4: 0,
};

const SUMMONER_IFRIT_SPAWN: SummonerPetSpawnSpec = SummonerPetSpawnSpec {
    pet_id: SUMMONER_PET_HOTBAR_IFRIT,
    base_id: 0x34C1,
    name_id: 10262,
    name: "红宝石伊弗利特",
    model_chara: 3122,
    display_flags: 0x0004_002B,
    parameter_unk3: 0,
    parameter_unk4: 0,
};

const SUMMONER_TITAN_SPAWN: SummonerPetSpawnSpec = SummonerPetSpawnSpec {
    pet_id: SUMMONER_PET_HOTBAR_TITAN,
    base_id: 0x34C3,
    name_id: 10264,
    name: "黄宝石泰坦",
    model_chara: 3124,
    display_flags: 0x0004_002B,
    parameter_unk3: 0,
    parameter_unk4: 0,
};

const SUMMONER_GARUDA_SPAWN: SummonerPetSpawnSpec = SummonerPetSpawnSpec {
    pet_id: SUMMONER_PET_HOTBAR_GARUDA,
    base_id: 0x34C2,
    name_id: 10263,
    name: "绿宝石迦楼罗",
    model_chara: 3123,
    display_flags: 0x0004_002B,
    parameter_unk3: 0,
    parameter_unk4: 0,
};

#[derive(Debug, Clone, Copy)]
struct SummonerPrimalTransitionSpec {
    spawn: SummonerPetSpawnSpec,
    finisher_action_id: u32,
}

struct DemiAutoAttackPlan {
    owner_id: ObjectId,
    pet_id: ObjectId,
    target_id: ObjectId,
    action_id: u32,
    potency: u32,
}

fn send_job_gauge_update(
    network: &mut NetworkState,
    from_actor_id: ObjectId,
    classjob_id: u8,
    data: u64,
) {
    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorGauge { classjob_id, data });
    network.send_to_by_actor_id(
        from_actor_id,
        FromServer::PacketSegment(ipc, from_actor_id),
        DestinationNetwork::ZoneClients,
    );
}

fn send_summoner_pet_parameters(network: &mut NetworkState, owner_actor_id: ObjectId, pet_id: u32) {
    send_summoner_pet_parameters_with_flags(network, owner_actor_id, pet_id, 5, 7);
}

fn summoner_pet_parameter_flags(pet_id: u32) -> (u32, u32) {
    match pet_id {
        SUMMONER_PET_HOTBAR_CARBUNCLE => (5, 7),
        SUMMONER_PET_HOTBAR_SOLAR_BAHAMUT
        | SUMMONER_PET_HOTBAR_BAHAMUT
        | SUMMONER_PET_HOTBAR_IFRIT
        | SUMMONER_PET_HOTBAR_TITAN
        | SUMMONER_PET_HOTBAR_GARUDA => (0, 0),
        _ => (5, 7),
    }
}

fn send_summoner_pet_parameters_with_flags(
    network: &mut NetworkState,
    owner_actor_id: ObjectId,
    pet_id: u32,
    unk3: u32,
    unk4: u32,
) {
    send_summoner_pet_parameters_with_mount_state(network, owner_actor_id, pet_id, unk3, unk4, 0);
}

fn send_summoner_pet_parameters_with_mount_state(
    network: &mut NetworkState,
    owner_actor_id: ObjectId,
    pet_id: u32,
    unk3: u32,
    unk4: u32,
    mount_state: u32,
) {
    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::SetPetParameters {
            pet_id,
            unk2: 2,
            unk3,
            unk4,
            mount_state,
        }),
        DestinationNetwork::ZoneClients,
    );
}

fn clear_summoner_pet_binding(
    network: &mut NetworkState,
    owner_actor_id: ObjectId,
    unk3: u32,
    unk4: u32,
) {
    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::SetupPet {
            owner_id: owner_actor_id,
            pet_id: 0,
            pet_actor_id: SUMMONER_INVALID_PET_ACTOR_ID,
            unk2: 1,
            unk3: 1,
        }),
        DestinationNetwork::ZoneClients,
    );
    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::SetPetParameters {
            pet_id: 0,
            unk2: 2,
            unk3,
            unk4,
            mount_state: 0,
        }),
        DestinationNetwork::ZoneClients,
    );
}

fn send_demi_summon_revert_packets(network: &mut NetworkState, owner_actor_id: ObjectId) {
    clear_summoner_pet_binding(network, owner_actor_id, 0, 0);
    send_summoner_pet_parameters(network, owner_actor_id, SUMMONER_PET_HOTBAR_CARBUNCLE);
}

fn demi_summon_spawn_for_action(action_id: u32) -> Option<SummonerPetSpawnSpec> {
    match action_id {
        ACTION_SUMMON_SOLAR_BAHAMUT => Some(SUMMONER_SOLAR_BAHAMUT_SPAWN),
        ACTION_SUMMON_BAHAMUT => Some(SUMMONER_BAHAMUT_SPAWN),
        _ => None,
    }
}

fn elemental_primal_transition_for_action(action_id: u32) -> Option<SummonerPrimalTransitionSpec> {
    match action_id {
        ACTION_SUMMON_IFRIT_II => Some(SummonerPrimalTransitionSpec {
            spawn: SUMMONER_IFRIT_SPAWN,
            finisher_action_id: ACTION_INFERNO,
        }),
        ACTION_SUMMON_TITAN_II => Some(SummonerPrimalTransitionSpec {
            spawn: SUMMONER_TITAN_SPAWN,
            finisher_action_id: ACTION_EARTHEN_FURY,
        }),
        ACTION_SUMMON_GARUDA_II => Some(SummonerPrimalTransitionSpec {
            spawn: SUMMONER_GARUDA_SPAWN,
            finisher_action_id: ACTION_AERIAL_BLAST,
        }),
        _ => None,
    }
}

pub(crate) fn has_pet_transition_for_action(action_id: u32) -> bool {
    demi_summon_spawn_for_action(action_id).is_some()
        || elemental_primal_transition_for_action(action_id).is_some()
}

pub(crate) fn is_demi_summon(action_id: u32) -> bool {
    demi_summon_spawn_for_action(action_id).is_some()
}

pub(crate) fn is_elemental_primal_summon(action_id: u32) -> bool {
    elemental_primal_transition_for_action(action_id).is_some()
}

pub(crate) fn augment_action_result_effects(action_id: u32, effects: &mut Vec<ActionEffect>) {
    match action_id {
        ACTION_SUMMON_IFRIT_II => {
            insert_primal_summon_action_effect(effects, 1);
            insert_ready_status_action_effect(effects, STATUS_CRIMSON_CYCLONE_READY);
        }
        ACTION_SUMMON_TITAN_II => {
            insert_primal_summon_action_effect(effects, 2);
        }
        ACTION_SUMMON_GARUDA_II => {
            insert_primal_summon_action_effect(effects, 3);
            insert_ready_status_action_effect(effects, STATUS_SLIPSTREAM_READY);
        }
        _ => {}
    }
}

fn insert_primal_summon_action_effect(effects: &mut Vec<ActionEffect>, elemental_index: u8) {
    if effects.iter().any(|effect| {
        matches!(
            effect.kind,
            EffectKind::SummonPet {
                unk: [index, 0, 0, 0, 128, 10, 1]
            } if index == elemental_index
        )
    }) {
        return;
    }

    insert_action_effect_before_combo(
        effects,
        ActionEffect {
            kind: EffectKind::SummonPet {
                unk: [elemental_index, 0, 0, 0, 128, 10, 1],
            },
        },
    );
}

fn insert_ready_status_action_effect(effects: &mut Vec<ActionEffect>, status_id: u16) {
    if effects.iter().any(|effect| {
        matches!(
            effect.kind,
            EffectKind::GainEffectSelf {
                effect_id,
                ..
            } if effect_id == status_id
        )
    }) {
        return;
    }

    insert_action_effect_before_combo(
        effects,
        ActionEffect {
            kind: EffectKind::GainEffectSelf {
                unk1: 0,
                unk2: 0,
                param: 0,
                unk3: 128,
                effect_id: status_id,
                duration: 0.0,
            },
        },
    );
}

fn insert_action_effect_before_combo(effects: &mut Vec<ActionEffect>, effect: ActionEffect) {
    if effects.len() >= 8 {
        tracing::warn!(
            "Skipping Summoner retail action effect because ActionResult already has {} effects",
            effects.len()
        );
        return;
    }

    let index = effects
        .iter()
        .position(|effect| matches!(effect.kind, EffectKind::ExecuteCombo { .. }))
        .unwrap_or(effects.len());
    effects.insert(index, effect);
}

fn send_initial_pet_status_list(
    network: &mut NetworkState,
    owner_actor_id: ObjectId,
    pet_actor_id: ObjectId,
    level: u8,
    health_points: u32,
    max_health_points: u32,
    resource_points: u16,
    max_resource_points: u16,
) {
    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::StatusEffectList(StatusEffectList {
        classjob_id: 0,
        level,
        unk1: level,
        unk2: 0,
        health_points,
        max_health_points,
        resource_points,
        max_resource_points,
        shield: 0,
        unk3: 0,
        statuses: [StatusEffect::default(); 30],
        unk4: 0,
    }));
    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::PacketSegment(ipc, pet_actor_id),
        DestinationNetwork::ZoneClients,
    );
}

pub(crate) fn send_retail_pet_reveal_controls(
    network: &mut NetworkState,
    instance: &mut Instance,
    pet_actor_id: ObjectId,
) {
    network.send_ac_in_range_inclusive_instance(
        instance,
        pet_actor_id,
        ActorControlCategory::Unknown {
            category: 36,
            param1: 1,
            param2: 142,
            param3: 0,
            param4: 0,
            param5: 0,
        },
    );
    network.send_ac_in_range_inclusive_instance(
        instance,
        pet_actor_id,
        ActorControlCategory::Unknown {
            category: 54,
            param1: 1,
            param2: 0,
            param3: 0,
            param4: 0,
            param5: 0,
        },
    );

    if let Some(actor) = instance.find_actor_mut(pet_actor_id) {
        actor
            .get_common_spawn_mut()
            .display_flags
            .remove(DisplayFlag::INVISIBLE);
    }
}

pub(crate) fn sync_pet_for_mount(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
) {
    let pet_ids = live_summoner_pet_ids(instance, owner_actor_id);
    let Some(pet_id) = pet_ids
        .iter()
        .filter_map(|pet_actor_id| {
            instance
                .find_actor(*pet_actor_id)
                .map(|actor| actor.get_common_spawn().pet_id)
        })
        .next()
    else {
        return;
    };

    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::SetPetEntityId { unk1: 1 }),
        DestinationNetwork::ZoneClients,
    );

    let (unk3, unk4) = summoner_pet_parameter_flags(pet_id);
    send_summoner_pet_parameters_with_mount_state(network, owner_actor_id, pet_id, unk3, unk4, 1);

    for pet_actor_id in pet_ids {
        network.send_in_range_inclusive_instance(
            pet_actor_id,
            instance,
            FromServer::ActorControl(
                pet_actor_id,
                ActorControlCategory::Unknown {
                    category: 266,
                    param1: 0,
                    param2: 0,
                    param3: 0,
                    param4: 0,
                    param5: 0,
                },
            ),
            DestinationNetwork::ZoneClients,
        );
        network.send_in_range_inclusive_instance(
            pet_actor_id,
            instance,
            FromServer::ActorControl(
                pet_actor_id,
                ActorControlCategory::Unknown {
                    category: 54,
                    param1: 0,
                    param2: 0,
                    param3: 0,
                    param4: 0,
                    param5: 0,
                },
            ),
            DestinationNetwork::ZoneClients,
        );
    }

    send_summoner_pet_parameters_with_mount_state(network, owner_actor_id, pet_id, unk3, unk4, 1);
}

pub(crate) fn sync_pet_after_dismount(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
) {
    let pet_ids = live_summoner_pet_ids(instance, owner_actor_id);
    let Some(pet_id) = pet_ids
        .iter()
        .filter_map(|pet_actor_id| {
            instance
                .find_actor(*pet_actor_id)
                .map(|actor| actor.get_common_spawn().pet_id)
        })
        .next()
    else {
        return;
    };

    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::SetPetEntityId { unk1: 0 }),
        DestinationNetwork::ZoneClients,
    );

    for pet_actor_id in pet_ids {
        network.send_in_range_inclusive_instance(
            pet_actor_id,
            instance,
            FromServer::ActorControl(
                pet_actor_id,
                ActorControlCategory::Unknown {
                    category: 267,
                    param1: 0,
                    param2: 0,
                    param3: 0,
                    param4: 0,
                    param5: 0,
                },
            ),
            DestinationNetwork::ZoneClients,
        );
        network.send_in_range_inclusive_instance(
            pet_actor_id,
            instance,
            FromServer::ActorControl(
                pet_actor_id,
                ActorControlCategory::Unknown {
                    category: 54,
                    param1: 1,
                    param2: 0,
                    param3: 0,
                    param4: 0,
                    param5: 0,
                },
            ),
            DestinationNetwork::ZoneClients,
        );
    }

    let (unk3, unk4) = summoner_pet_parameter_flags(pet_id);
    send_summoner_pet_parameters_with_flags(network, owner_actor_id, pet_id, unk3, unk4);
}

pub(crate) fn update_summoner_gauge_if_needed(
    network: &mut NetworkState,
    actor: &NetworkedActor,
    actor_id: ObjectId,
) {
    let class_job = actor.get_common_spawn().class_job;
    if !is_summoner(class_job) {
        return;
    }

    let NetworkedActor::Player { combat_state, .. } = actor else {
        return;
    };

    send_job_gauge_update(
        network,
        actor_id,
        class_job,
        build_summoner_gauge_data(combat_state, actor.get_common_spawn().level),
    );
}

pub(crate) fn dismiss_pet(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
) -> bool {
    let pet_ids: Vec<ObjectId> = instance
        .actors
        .iter()
        .filter_map(|(id, actor)| match actor {
            NetworkedActor::Npc { spawn, .. } if spawn.common.owner_id == owner_actor_id => {
                Some(*id)
            }
            _ => None,
        })
        .collect();

    if pet_ids.is_empty() {
        return false;
    }

    if let Some(NetworkedActor::Player { combat_state, .. }) =
        instance.find_actor_mut(owner_actor_id)
    {
        combat_state.summoner.carbuncle_summoned = false;
    }

    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::SetPetEntityId { unk1: 0 }),
        DestinationNetwork::ZoneClients,
    );
    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::SetupPet {
            owner_id: owner_actor_id,
            pet_id: 0,
            pet_actor_id: ObjectId::default(),
            unk2: 0,
            unk3: 0,
        }),
        DestinationNetwork::ZoneClients,
    );
    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::SetPetParameters {
            pet_id: 0,
            unk2: 0,
            unk3: 0,
            unk4: 7,
            mount_state: 0,
        }),
        DestinationNetwork::ZoneClients,
    );

    if let Some(actor) = instance.find_actor(owner_actor_id) {
        update_summoner_gauge_if_needed(network, actor, owner_actor_id);
    }

    for pet_id in pet_ids {
        instance.cancel_actor_tasks(pet_id);
        mark_pet_dead(instance, pet_id);

        set_character_mode(instance, network, pet_id, CharacterMode::Dead, 0);
        network.send_ac_in_range_inclusive_instance(
            instance,
            pet_id,
            ActorControlCategory::Kill { animation_id: 0 },
        );
        instance.insert_task(
            ClientId::default(),
            pet_id,
            PET_DISMISS_FADE_OUT,
            QueuedTaskData::DeadFadeOut { actor_id: pet_id },
        );
    }

    true
}

pub(crate) fn apply_pet_command(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
    command: PetCommand,
) -> bool {
    match command {
        PetCommand::Recall => dismiss_pet(network, instance, owner_actor_id),
        PetCommand::Follow | PetCommand::Place(_) | PetCommand::Stay => {
            let owner_position = instance.find_actor(owner_actor_id).map(|actor| {
                (
                    actor.get_common_spawn().position,
                    actor.get_common_spawn().rotation,
                )
            });
            let pet_ids: Vec<ObjectId> = instance
                .actors
                .iter()
                .filter_map(|(id, actor)| match actor {
                    NetworkedActor::Npc { spawn, .. }
                        if spawn.common.owner_id == owner_actor_id =>
                    {
                        Some(*id)
                    }
                    _ => None,
                })
                .collect();

            if pet_ids.is_empty() {
                return false;
            }

            let mut moves = Vec::new();
            for pet_id in pet_ids {
                let Some(actor) = instance.find_actor_mut(pet_id) else {
                    continue;
                };

                let NetworkedActor::Npc {
                    state,
                    navmesh_target,
                    navmesh_path,
                    navmesh_path_lerp,
                    last_position,
                    spawn,
                    ..
                } = actor
                else {
                    continue;
                };

                navmesh_path.clear();
                *navmesh_path_lerp = 0.0;
                *last_position = None;

                match command {
                    PetCommand::Follow => {
                        *state = NpcState::Follow;
                        *navmesh_target = Some(owner_actor_id);
                        spawn.common.target_id = ObjectTypeId::default();
                        if let Some((owner_position, owner_rotation)) = owner_position {
                            let mut position = owner_position;
                            position.0.x += owner_rotation.sin() * SUMMONER_PET_SPAWN_DISTANCE;
                            position.0.z += owner_rotation.cos() * SUMMONER_PET_SPAWN_DISTANCE;
                            let rotation =
                                rotate_towards(position.0, owner_position.0, owner_rotation);
                            spawn.common.position = position;
                            spawn.common.rotation = rotation;
                            moves.push((pet_id, position, rotation));
                        }
                    }
                    PetCommand::Place(position) => {
                        *state = NpcState::Stay;
                        *navmesh_target = None;
                        spawn.common.target_id = ObjectTypeId::default();
                        let rotation = owner_position
                            .map(|(owner_position, _)| {
                                rotate_towards(position.0, owner_position.0, spawn.common.rotation)
                            })
                            .unwrap_or(spawn.common.rotation);
                        spawn.common.position = position;
                        spawn.common.rotation = rotation;
                        moves.push((pet_id, position, rotation));
                    }
                    PetCommand::Stay => {
                        *state = NpcState::Stay;
                        *navmesh_target = None;
                        spawn.common.target_id = ObjectTypeId::default();
                        moves.push((pet_id, spawn.common.position, spawn.common.rotation));
                    }
                    PetCommand::Recall => unreachable!(),
                }
            }

            for (pet_id, position, rotation) in moves {
                network.send_in_range_instance(
                    pet_id,
                    instance,
                    FromServer::ActorMove(
                        pet_id,
                        position,
                        rotation,
                        MoveAnimationType::empty(),
                        MoveAnimationState::empty(),
                        JumpState::empty(),
                    ),
                    DestinationNetwork::ZoneClients,
                );
            }

            true
        }
    }
}

fn rotate_towards(from_pos: Vec3, to_pos: Vec3, fallback: f32) -> f32 {
    let delta_x = to_pos.x - from_pos.x;
    let delta_z = to_pos.z - from_pos.z;
    if delta_x.abs() < f32::EPSILON && delta_z.abs() < f32::EPSILON {
        return fallback;
    }

    let rotation = f32::atan2(delta_x, delta_z);
    if rotation >= std::f32::consts::PI {
        -std::f32::consts::PI
    } else {
        rotation
    }
}

fn retail_carbuncle_spawn_template() -> Option<SpawnNpc> {
    static TEMPLATE: OnceLock<Option<SpawnNpc>> = OnceLock::new();

    TEMPLATE
        .get_or_init(|| {
            let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            let path = manifest_dir.join("../../resources/data/tests/zone/server/npc_spawn.bin");
            let buffer = std::fs::read(path).ok()?;
            let mut cursor = std::io::Cursor::new(buffer);
            SpawnNpc::read_le(&mut cursor).ok()
        })
        .clone()
}

fn live_summoner_pet_ids(instance: &Instance, owner_actor_id: ObjectId) -> Vec<ObjectId> {
    instance
        .actors
        .iter()
        .filter_map(|(id, actor)| match actor {
            NetworkedActor::Npc { spawn, state, .. }
                if spawn.common.owner_id == owner_actor_id && *state != NpcState::Dead =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect()
}

fn mark_pet_dead(instance: &mut Instance, pet_actor_id: ObjectId) {
    if let Some(NetworkedActor::Npc {
        state,
        hate_list,
        spawn,
        ..
    }) = instance.find_actor_mut(pet_actor_id)
    {
        *state = NpcState::Dead;
        hate_list.clear();
        spawn.common.target_id = ObjectTypeId::default();
        spawn.common.health_points = 0;
    }
}

fn kill_summoner_pet_for_transition(
    network: &mut NetworkState,
    instance: &mut Instance,
    pet_actor_id: ObjectId,
) {
    instance.cancel_actor_tasks(pet_actor_id);
    mark_pet_dead(instance, pet_actor_id);

    network.send_ac_in_range_inclusive_instance(
        instance,
        pet_actor_id,
        ActorControlCategory::Kill { animation_id: 0 },
    );
    set_character_mode(instance, network, pet_actor_id, CharacterMode::Dead, 0);
    instance.insert_task(
        ClientId::default(),
        pet_actor_id,
        PET_TRANSITION_FADE_OUT,
        QueuedTaskData::DeadFadeOut {
            actor_id: pet_actor_id,
        },
    );
}

fn kill_live_summoner_pets_for_transition(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
) {
    for pet_actor_id in live_summoner_pet_ids(instance, owner_actor_id) {
        kill_summoner_pet_for_transition(network, instance, pet_actor_id);
    }
}

fn prepare_demi_summon_transition(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
    demi_spec: SummonerPetSpawnSpec,
) {
    clear_summoner_pet_binding(network, owner_actor_id, 5, 7);
    kill_live_summoner_pets_for_transition(network, instance, owner_actor_id);
    send_summoner_pet_parameters_with_flags(
        network,
        owner_actor_id,
        demi_spec.pet_id,
        demi_spec.parameter_unk3,
        demi_spec.parameter_unk4,
    );
}

fn prepare_elemental_primal_transition(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
    primal_spec: SummonerPetSpawnSpec,
) {
    clear_summoner_pet_binding(network, owner_actor_id, 0, 0);
    kill_live_summoner_pets_for_transition(network, instance, owner_actor_id);
    send_summoner_pet_parameters_with_flags(
        network,
        owner_actor_id,
        primal_spec.pet_id,
        primal_spec.parameter_unk3,
        primal_spec.parameter_unk4,
    );
}

pub(crate) fn prepare_pet_transition_for_action(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
    action_id: u32,
) {
    if let Some(demi_spec) = demi_summon_spawn_for_action(action_id) {
        prepare_demi_summon_transition(network, instance, owner_actor_id, demi_spec);
    } else if let Some(primal_spec) = elemental_primal_transition_for_action(action_id) {
        prepare_elemental_primal_transition(network, instance, owner_actor_id, primal_spec.spawn);
    }
}

fn send_pet_set_target(
    network: &mut NetworkState,
    instance: &Instance,
    pet_actor_id: ObjectId,
    target: ObjectTypeId,
) {
    network.send_in_range_instance(
        pet_actor_id,
        instance,
        FromServer::ActorControlTarget(pet_actor_id, target, ActorControlCategory::SetTarget {}),
        DestinationNetwork::ZoneClients,
    );
}

fn spawn_summoner_pet_actor(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
    spec: SummonerPetSpawnSpec,
) -> Option<ObjectId> {
    let actor = instance.find_actor(owner_actor_id)?;
    let level = actor.get_common_spawn().level;
    let owner_position = actor.position();
    let owner_rotation = actor.rotation();
    let owner_common = actor.get_common_spawn();
    let pet_hp = owner_common.max_health_points;
    let pet_mp = owner_common.max_resource_points;

    let mut pet_position = owner_position;
    pet_position.0.x += owner_rotation.sin() * SUMMONER_PET_SPAWN_DISTANCE;
    pet_position.0.z += owner_rotation.cos() * SUMMONER_PET_SPAWN_DISTANCE;
    let pet_rotation = rotate_towards(pet_position.0, owner_position.0, owner_rotation);
    let pet_actor_id = Instance::generate_actor_id();

    network.send_to_by_actor_id(
        owner_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::SetupPet {
            owner_id: owner_actor_id,
            pet_id: spec.pet_id,
            pet_actor_id,
            unk2: 1,
            unk3: 1,
        }),
        DestinationNetwork::ZoneClients,
    );
    send_initial_pet_status_list(
        network,
        owner_actor_id,
        pet_actor_id,
        level,
        pet_hp,
        pet_hp,
        pet_mp,
        pet_mp,
    );
    send_summoner_pet_parameters_with_flags(
        network,
        owner_actor_id,
        spec.pet_id,
        spec.parameter_unk3,
        spec.parameter_unk4,
    );

    let mut spawn = retail_carbuncle_spawn_template().unwrap_or_default();
    spawn.common.base_id = spec.base_id;
    spawn.common.name_id = spec.name_id;
    spawn.common.name = spec.name.to_string();
    spawn.common.pet_id = spec.pet_id;
    spawn.common.owner_id = owner_actor_id;
    spawn.common.max_health_points = pet_hp;
    spawn.common.health_points = pet_hp;
    spawn.common.resource_points = pet_mp;
    spawn.common.max_resource_points = pet_mp;
    spawn.common.model_chara = spec.model_chara;
    spawn.common.object_kind = ObjectKind::BattleNpc(BattleNpcSubKind::Pet);
    spawn.common.level = level;
    spawn.common.position = pet_position;
    spawn.common.rotation = pet_rotation;
    spawn.common.display_flags = DisplayFlag(spec.display_flags);
    spawn.common.layout_id = 0;
    spawn.common.handler_id = Default::default();
    spawn.common.target_id = Default::default();
    spawn.common.combat_tagger_id = Default::default();
    spawn.common.tether_target_id = Default::default();
    spawn.character_data_flags = CharacterDataFlag::from_bits_retain(0x1);
    spawn.character_data_icon = 0;

    instance.insert_npc(pet_actor_id, spawn);

    if let Some(from_id) = network.find_by_actor(owner_actor_id) {
        instance.insert_task(
            from_id,
            owner_actor_id,
            PET_REVEAL_DELAY,
            QueuedTaskData::RevealPet {
                actor_id: pet_actor_id,
            },
        );
    }

    Some(pet_actor_id)
}

fn spawn_elemental_primal_actor(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
    spec: SummonerPrimalTransitionSpec,
    preferred_target_id: ObjectId,
) -> Option<ObjectId> {
    let pet_actor_id = spawn_summoner_pet_actor(network, instance, owner_actor_id, spec.spawn)?;

    if preferred_target_id.is_valid() {
        let target = ObjectTypeId {
            object_id: preferred_target_id,
            object_type: ObjectTypeKind::None,
        };
        if let Some(pet) = instance.find_actor_mut(pet_actor_id) {
            pet.get_common_spawn_mut().target_id = target;
        }
        send_pet_set_target(network, instance, pet_actor_id, target);
    }

    instance.insert_task(
        ClientId::default(),
        pet_actor_id,
        SUMMONER_PRIMAL_FINISHER_DELAY,
        QueuedTaskData::SummonerPrimalFinisher {
            owner_id: owner_actor_id,
            pet_id: pet_actor_id,
            preferred_target_id,
            action_id: spec.finisher_action_id,
            potency: SUMMONER_PRIMAL_FINISHER_POTENCY,
            expires_at: Instant::now() + SUMMONER_PRIMAL_WAIT_FOR_TARGET_DURATION,
        },
    );

    Some(pet_actor_id)
}

pub(crate) fn spawn_pet_after_action(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
    action_id: u32,
    preferred_target_id: ObjectId,
) {
    if let Some(demi_spec) = demi_summon_spawn_for_action(action_id) {
        let _ = spawn_summoner_pet_actor(network, instance, owner_actor_id, demi_spec);
    } else if let Some(primal_spec) = elemental_primal_transition_for_action(action_id) {
        let _ = spawn_elemental_primal_actor(
            network,
            instance,
            owner_actor_id,
            primal_spec,
            preferred_target_id,
        );
    }
}

pub(crate) fn apply_demi_summon_revert(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
    gauge_update: Option<(u8, u64)>,
) {
    kill_live_summoner_pets_for_transition(network, instance, owner_actor_id);
    send_demi_summon_revert_packets(network, owner_actor_id);

    if let Some(NetworkedActor::Player { combat_state, .. }) =
        instance.find_actor_mut(owner_actor_id)
    {
        combat_state.summoner.carbuncle_summoned = true;
    }

    if let Some((classjob_id, data)) = gauge_update {
        send_job_gauge_update(network, owner_actor_id, classjob_id, data);
    }

    let _ = spawn_summoner_pet_actor(network, instance, owner_actor_id, SUMMONER_CARBUNCLE_SPAWN);
}

pub(crate) fn apply_elemental_primal_revert(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_actor_id: ObjectId,
    pet_id: ObjectId,
) {
    let pet_is_current = matches!(
        instance.find_actor(pet_id),
        Some(NetworkedActor::Npc { spawn, state, .. })
            if spawn.common.owner_id == owner_actor_id && *state != NpcState::Dead
    );
    if !pet_is_current {
        return;
    }

    kill_summoner_pet_for_transition(network, instance, pet_id);
    send_demi_summon_revert_packets(network, owner_actor_id);
    if let Some(NetworkedActor::Player { combat_state, .. }) =
        instance.find_actor_mut(owner_actor_id)
    {
        combat_state.summoner.carbuncle_summoned = true;
        clear_primal_summon_timer(&mut combat_state.summoner);
    }
    if let Some(actor) = instance.find_actor(owner_actor_id) {
        update_summoner_gauge_if_needed(network, actor, owner_actor_id);
    }
    let _ = spawn_summoner_pet_actor(network, instance, owner_actor_id, SUMMONER_CARBUNCLE_SPAWN);
}

fn live_attackable_npc(instance: &Instance, actor_id: ObjectId) -> bool {
    matches!(
        instance.find_actor(actor_id),
        Some(NetworkedActor::Npc {
            state,
            spawn,
            ..
        }) if *state != NpcState::Dead
            && spawn.common.mode != CharacterMode::Dead
            && spawn.common.health_points > 0
            && !spawn.common.owner_id.is_valid()
    )
}

fn resolve_summoner_pet_attack_target(
    instance: &Instance,
    owner_id: ObjectId,
    pet_id: ObjectId,
    preferred_target_id: ObjectId,
) -> Option<ObjectId> {
    if preferred_target_id.is_valid() && live_attackable_npc(instance, preferred_target_id) {
        return Some(preferred_target_id);
    }

    let pet_target = instance
        .find_actor(pet_id)
        .map(|actor| actor.get_common_spawn().target_id.object_id)
        .filter(|target_id| target_id.is_valid() && live_attackable_npc(instance, *target_id));
    if pet_target.is_some() {
        return pet_target;
    }

    let owner_target = instance
        .find_actor(owner_id)
        .map(|actor| actor.get_common_spawn().target_id.object_id)
        .filter(|target_id| target_id.is_valid() && live_attackable_npc(instance, *target_id));
    if owner_target.is_some() {
        return owner_target;
    }

    instance
        .actors
        .iter()
        .filter_map(|(id, actor)| match actor {
            NetworkedActor::Npc {
                state,
                spawn,
                hate_list,
                ..
            } if *state != NpcState::Dead
                && spawn.common.health_points > 0
                && !spawn.common.owner_id.is_valid()
                && hate_list.contains_key(&owner_id) =>
            {
                Some((*id, hate_list.get(&owner_id).copied().unwrap_or_default()))
            }
            _ => None,
        })
        .max_by_key(|(_, hate)| *hate)
        .map(|(id, _)| id)
}

fn pet_facing(
    instance: &Instance,
    pet_id: ObjectId,
    target_id: ObjectId,
) -> Option<(Position, f32)> {
    let pet = instance.find_actor(pet_id)?;
    let pet_common = pet.get_common_spawn();
    let pet_position = pet_common.position;
    let target_position = instance.find_actor(target_id)?.position();
    Some((
        pet_position,
        rotate_towards(pet_position.0, target_position.0, pet_common.rotation),
    ))
}

fn execute_summoner_pet_magic_attack(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_id: ObjectId,
    pet_id: ObjectId,
    target_id: ObjectId,
    action_id: u32,
    potency: u32,
    animation_lock: f32,
    kill_pet_after: bool,
) -> bool {
    if !live_attackable_npc(instance, target_id) {
        return false;
    }

    let Some(base_damage) = instance.find_actor(owner_id).and_then(|owner| {
        if let NetworkedActor::Player { parameters, .. } = owner {
            Some(parameters.calc_magical_damage(potency))
        } else {
            None
        }
    }) else {
        return false;
    };
    let Some((pet_position, pet_rotation)) = pet_facing(instance, pet_id, target_id) else {
        return false;
    };

    let (mut damage, damage_kind) = instance
        .find_actor(owner_id)
        .and_then(|owner| {
            if let NetworkedActor::Player { parameters, .. } = owner {
                Some(parameters.roll_damage(base_damage))
            } else {
                None
            }
        })
        .unwrap_or((base_damage, Default::default()));

    if let Some(NetworkedActor::Player { parameters, .. }) = instance.find_actor(target_id) {
        let mitigation = parameters.mitigation_against(true);
        damage = ((damage as f64) * (1.0 - mitigation)).floor() as u32;
    }
    if damage == 0 {
        tracing::warn!(
            "Summoner pet attack {} from {} to {} produced zero damage at potency {}",
            action_id,
            pet_id,
            target_id,
            potency
        );
        return false;
    }

    if let Some(target) = instance.find_actor_mut(target_id) {
        if let Some(hate_list) = target.npc_hate_list_mut() {
            let entry = hate_list.entry(owner_id).or_insert(0);
            *entry = entry.saturating_add(damage);
        }

        target.apply_damage(damage);
    }

    let target = ObjectTypeId {
        object_id: target_id,
        object_type: ObjectTypeKind::None,
    };
    if let Some(pet) = instance.find_actor_mut(pet_id) {
        let common = pet.get_common_spawn_mut();
        common.target_id = target;
        common.rotation = pet_rotation;
    }

    let mut effects = [ActionEffect::default(); 8];
    effects[0] = ActionEffect {
        kind: EffectKind::Damage {
            amount: damage,
            damage_kind,
            damage_type: DamageType::Magic,
            damage_element: DamageElement::Unaspected,
            bonus_percent: 0,
            unk3: 0,
            unk4: 0,
        },
    };
    effects[1] = ActionEffect {
        kind: EffectKind::ExecuteCombo {
            sequence: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            unk5: 128,
            action_id: action_id as u16,
        },
    };

    network.send_in_range_inclusive_instance(
        pet_id,
        instance,
        FromServer::ActorMove(
            pet_id,
            pet_position,
            pet_rotation,
            MoveAnimationType::empty(),
            MoveAnimationState::empty(),
            JumpState::empty(),
        ),
        DestinationNetwork::ZoneClients,
    );

    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(ActionResult {
        animation_target_id: target,
        target_id_again: target,
        action_id,
        animation_lock,
        rotation: pet_rotation,
        spell_id: action_id as u16,
        source_sequence: 0,
        effect_count: 1,
        effects,
        action_type: ActionType::Action,
        global_sequence: network.global_action_sequence,
        ..Default::default()
    }));
    network.global_action_sequence += 1;
    network.send_in_range_inclusive_instance(
        pet_id,
        instance,
        FromServer::PacketSegment(ipc, pet_id),
        DestinationNetwork::ZoneClients,
    );

    if kill_pet_after {
        kill_summoner_pet_for_transition(network, instance, pet_id);
    }

    true
}

pub(crate) fn process_elemental_primal_finisher(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    owner_id: ObjectId,
    pet_id: ObjectId,
    preferred_target_id: ObjectId,
    action_id: u32,
    potency: u32,
    expires_at: Instant,
) -> Option<ObjectId> {
    let pet_is_current = matches!(
        instance.find_actor(pet_id),
        Some(NetworkedActor::Npc { spawn, state, .. })
            if spawn.common.owner_id == owner_id && *state != NpcState::Dead
    );
    if !pet_is_current {
        return None;
    }

    if Instant::now() >= expires_at {
        let mut network = network.lock();
        apply_elemental_primal_revert(&mut network, instance, owner_id, pet_id);
        return None;
    }

    if let Some(target_id) =
        resolve_summoner_pet_attack_target(instance, owner_id, pet_id, preferred_target_id)
    {
        let executed = {
            let mut network = network.lock();
            execute_summoner_pet_magic_attack(
                &mut network,
                instance,
                owner_id,
                pet_id,
                target_id,
                action_id,
                potency,
                SUMMONER_PRIMAL_FINISHER_ANIMATION_LOCK,
                false,
            )
        };
        if executed {
            instance.insert_task(
                ClientId::default(),
                pet_id,
                Duration::from_secs_f32(SUMMONER_PRIMAL_FINISHER_ANIMATION_LOCK),
                QueuedTaskData::SummonerPrimalRevert { owner_id, pet_id },
            );
        }
        return executed.then_some(target_id);
    }

    instance.insert_task(
        ClientId::default(),
        pet_id,
        SUMMONER_PRIMAL_FINISHER_RETRY_DELAY
            .min(expires_at.saturating_duration_since(Instant::now())),
        QueuedTaskData::SummonerPrimalFinisher {
            owner_id,
            pet_id,
            preferred_target_id,
            action_id,
            potency,
            expires_at,
        },
    );
    None
}

pub(crate) fn register_slipstream_lingering_aoe_after_action(
    instance: &mut Instance,
    owner_id: ObjectId,
    action_id: u32,
    target_id: ObjectId,
) {
    if action_id != ACTION_SLIPSTREAM {
        return;
    }

    let Some(center) = instance.find_actor(target_id).map(|actor| actor.position()) else {
        return;
    };

    instance.insert_task(
        ClientId::default(),
        owner_id,
        SUMMONER_SLIPSTREAM_GROUND_VFX_DELAY,
        QueuedTaskData::SummonerSlipstreamGroundVfx { owner_id, center },
    );
    instance.insert_task(
        ClientId::default(),
        owner_id,
        SUMMONER_SLIPSTREAM_GROUND_TICK_INTERVAL,
        QueuedTaskData::SummonerSlipstreamTick {
            owner_id,
            center,
            radius: SUMMONER_SLIPSTREAM_GROUND_RADIUS,
            potency: SUMMONER_SLIPSTREAM_GROUND_POTENCY,
            ticks_remaining: SUMMONER_SLIPSTREAM_GROUND_TICKS,
        },
    );
}

pub(crate) fn spawn_slipstream_ground_vfx(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_id: ObjectId,
    center: Position,
    object_id: ObjectId,
) {
    let spawn = SpawnObject {
        kind: ObjectKind::AreaObject,
        targetable_status: 4,
        base_id: SUMMONER_SLIPSTREAM_GROUND_VFX_ID,
        entity_id: object_id,
        owner_id,
        radius: SUMMONER_SLIPSTREAM_GROUND_RADIUS,
        position: center,
        ..Default::default()
    };

    instance.insert_object(object_id, spawn, String::default());
    network.spawn_inserted_object_in_range(instance, object_id);
}

pub(crate) fn despawn_slipstream_ground_vfx(
    network: &mut NetworkState,
    instance: &mut Instance,
    object_id: ObjectId,
) {
    network.remove_actor(instance, object_id);
}

pub(crate) fn slipstream_ground_vfx_duration() -> Duration {
    SUMMONER_SLIPSTREAM_GROUND_VFX_DURATION
}

pub(crate) fn process_slipstream_lingering_tick(
    network: &mut NetworkState,
    instance: &mut Instance,
    owner_id: ObjectId,
    center: Position,
    radius: f32,
    potency: u32,
    ticks_remaining: u8,
) -> Vec<ObjectId> {
    if ticks_remaining == 0 {
        return Vec::new();
    }

    let Some(base_damage) = instance.find_actor(owner_id).and_then(|owner| {
        if let NetworkedActor::Player { parameters, .. } = owner {
            Some(parameters.calc_magical_damage(potency))
        } else {
            None
        }
    }) else {
        return Vec::new();
    };

    let targets: Vec<ObjectId> = instance
        .actors
        .iter()
        .filter_map(|(id, actor)| match actor {
            NetworkedActor::Npc { spawn, state, .. }
                if *state != NpcState::Dead
                    && !spawn.common.owner_id.is_valid()
                    && spawn.common.health_points > 0
                    && Vec3::distance(spawn.common.position.0, center.0) <= radius =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();

    let mut hit_targets = Vec::new();
    for target_id in targets {
        let (damage, _) = instance
            .find_actor(owner_id)
            .and_then(|owner| {
                if let NetworkedActor::Player { parameters, .. } = owner {
                    Some(parameters.roll_damage(base_damage))
                } else {
                    None
                }
            })
            .unwrap_or((base_damage, Default::default()));

        if let Some(target) = instance.find_actor_mut(target_id) {
            if let Some(hate_list) = target.npc_hate_list_mut() {
                let entry = hate_list.entry(owner_id).or_insert(0);
                *entry = entry.saturating_add(damage);
            }

            target.apply_damage(damage);
        } else {
            continue;
        }

        network.send_in_range_inclusive_instance(
            target_id,
            instance,
            FromServer::ActorControl(
                target_id,
                ActorControlCategory::TickDamage {
                    status_id: SUMMONER_SLIPSTREAM_TICK_STATUS_ID,
                    amount: damage,
                    source_actor_id: owner_id,
                    unk2: SUMMONER_SLIPSTREAM_TICK_UNK2,
                    unk3: 0,
                },
            ),
            DestinationNetwork::ZoneClients,
        );
        hit_targets.push(target_id);
    }

    let remaining = ticks_remaining.saturating_sub(1);
    if remaining > 0 {
        instance.insert_task(
            ClientId::default(),
            owner_id,
            SUMMONER_SLIPSTREAM_GROUND_TICK_INTERVAL,
            QueuedTaskData::SummonerSlipstreamTick {
                owner_id,
                center,
                radius,
                potency,
                ticks_remaining: remaining,
            },
        );
    }

    hit_targets
}

pub(crate) fn apply_summon_pet_effect(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    from_actor_id: ObjectId,
) {
    if let Some(NetworkedActor::Player { combat_state, .. }) =
        instance.find_actor_mut(from_actor_id)
    {
        combat_state.summoner.carbuncle_summoned = true;
    }

    let Some(actor) = instance.find_actor(from_actor_id) else {
        return;
    };
    let level = actor.get_common_spawn().level;
    let owner_position = actor.position();
    let owner_rotation = actor.rotation();
    let mut pet_position = owner_position;
    pet_position.0.x += owner_rotation.sin() * SUMMONER_PET_SPAWN_DISTANCE;
    pet_position.0.z += owner_rotation.cos() * SUMMONER_PET_SPAWN_DISTANCE;
    let pet_rotation = rotate_towards(pet_position.0, owner_position.0, owner_rotation);

    let pet_id = SUMMONER_PET_HOTBAR_CARBUNCLE;
    let pet_actor_id = ObjectId(fastrand::u32(..));

    let owner_common = actor.get_common_spawn();
    let pet_hp = owner_common.max_health_points;
    let pet_mp = owner_common.max_resource_points;

    let old_pet_ids: Vec<ObjectId> = instance
        .actors
        .iter()
        .filter_map(|(id, actor)| match actor {
            NetworkedActor::Npc { spawn, .. } if spawn.common.owner_id == from_actor_id => {
                Some(*id)
            }
            _ => None,
        })
        .collect();

    {
        let mut network = network.lock();
        for old_pet_id in &old_pet_ids {
            instance.cancel_actor_tasks(*old_pet_id);
            mark_pet_dead(instance, *old_pet_id);

            set_character_mode(instance, &mut network, *old_pet_id, CharacterMode::Dead, 0);
            network.send_ac_in_range_inclusive_instance(
                instance,
                *old_pet_id,
                ActorControlCategory::Kill { animation_id: 0 },
            );
            instance.insert_task(
                ClientId::default(),
                *old_pet_id,
                PET_DISMISS_FADE_OUT,
                QueuedTaskData::DeadFadeOut {
                    actor_id: *old_pet_id,
                },
            );
        }

        send_summoner_pet_parameters(&mut network, from_actor_id, pet_id);
        network.send_to_by_actor_id(
            from_actor_id,
            FromServer::ActorControlSelf(ActorControlCategory::SetupPet {
                owner_id: from_actor_id,
                pet_id,
                pet_actor_id,
                unk2: 1,
                unk3: 1,
            }),
            DestinationNetwork::ZoneClients,
        );
        send_summoner_pet_parameters(&mut network, from_actor_id, pet_id);
        send_initial_pet_status_list(
            &mut network,
            from_actor_id,
            pet_actor_id,
            level,
            pet_hp,
            pet_hp,
            pet_mp,
            pet_mp,
        );
    }

    let mut spawn = retail_carbuncle_spawn_template().unwrap_or_default();
    spawn.common.base_id = SUMMONER_CARBUNCLE_SPAWN.base_id;
    spawn.common.name_id = SUMMONER_CARBUNCLE_SPAWN.name_id;
    spawn.common.pet_id = pet_id;
    spawn.common.owner_id = from_actor_id;
    spawn.common.max_health_points = pet_hp;
    spawn.common.health_points = pet_hp;
    spawn.common.resource_points = pet_mp;
    spawn.common.max_resource_points = pet_mp;
    spawn.common.model_chara = SUMMONER_CARBUNCLE_SPAWN.model_chara;
    spawn.common.object_kind = ObjectKind::BattleNpc(BattleNpcSubKind::Pet);
    spawn.common.level = level;
    spawn.common.position = pet_position;
    spawn.common.rotation = pet_rotation;
    spawn.common.display_flags = DisplayFlag::UNK2 | DisplayFlag::INVISIBLE | DisplayFlag::UNK1;
    spawn.common.layout_id = 0;
    spawn.common.handler_id = Default::default();
    spawn.common.target_id = Default::default();
    spawn.common.combat_tagger_id = Default::default();
    spawn.common.tether_target_id = Default::default();
    spawn.character_data_flags = CharacterDataFlag::from_bits_retain(0x1);
    spawn.character_data_icon = 0;

    instance.insert_npc(pet_actor_id, spawn);

    if let Some(from_id) = network.lock().find_by_actor(from_actor_id) {
        instance.insert_task(
            from_id,
            from_actor_id,
            PET_REVEAL_DELAY,
            QueuedTaskData::RevealPet {
                actor_id: pet_actor_id,
            },
        );
    }
}

pub(crate) fn schedule_demi_auto_attack(instance: &mut Instance, owner_id: ObjectId) {
    instance.insert_task(
        ClientId::default(),
        owner_id,
        SUMMONER_DEMI_AUTO_ATTACK_INTERVAL,
        QueuedTaskData::SummonerDemiAutoAttack { owner_id },
    );
}

pub(crate) fn process_demi_auto_attack(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    owner_id: ObjectId,
) -> Option<ObjectId> {
    let Some(auto_attack) = instance.find_actor(owner_id).and_then(|actor| match actor {
        NetworkedActor::Player { combat_state, .. } => pending_demi_auto_attack(combat_state),
        _ => None,
    }) else {
        return None;
    };

    let Some(pet_id) = instance
        .actors
        .iter()
        .find_map(|(pet_id, pet_actor)| match pet_actor {
            NetworkedActor::Npc { spawn, state, .. }
                if spawn.common.owner_id == owner_id
                    && spawn.common.base_id == auto_attack.pet_base_id
                    && *state != NpcState::Dead
                    && spawn.common.health_points > 0 =>
            {
                Some(*pet_id)
            }
            _ => None,
        })
    else {
        return None;
    };

    let Some(target_id) =
        resolve_summoner_pet_attack_target(instance, owner_id, pet_id, ObjectId::default())
    else {
        schedule_demi_auto_attack(instance, owner_id);
        return None;
    };

    let plan = DemiAutoAttackPlan {
        owner_id,
        pet_id,
        target_id,
        action_id: auto_attack.action_id,
        potency: auto_attack.potency,
    };
    let executed = {
        let mut network = network.lock();
        execute_summoner_pet_magic_attack(
            &mut network,
            instance,
            plan.owner_id,
            plan.pet_id,
            plan.target_id,
            plan.action_id,
            plan.potency,
            DEMI_AUTO_ATTACK_ANIMATION_LOCK,
            false,
        )
    };

    if !executed {
        schedule_demi_auto_attack(instance, owner_id);
        return None;
    }

    if let Some(NetworkedActor::Player { combat_state, .. }) =
        instance.find_actor_mut(plan.owner_id)
    {
        mark_demi_auto_attack_used(combat_state);
    }

    let should_schedule_next = instance
        .find_actor(owner_id)
        .and_then(|actor| match actor {
            NetworkedActor::Player { combat_state, .. } => pending_demi_auto_attack(combat_state),
            _ => None,
        })
        .is_some();
    if should_schedule_next {
        schedule_demi_auto_attack(instance, owner_id);
    }

    Some(plan.target_id)
}

pub(crate) fn is_summoner(class_job: u8) -> bool {
    class_job == CLASSJOB_SUMMONER
}

fn summoner_window_active(smn: &SummonerState) -> bool {
    smn.attunement_expires_at
        .map(|expires_at| expires_at > Instant::now())
        .unwrap_or(true)
}

fn summoner_demi_active(smn: &SummonerState) -> bool {
    smn.demi_phase != SummonerDemiPhase::None
        && smn
            .demi_expires_at
            .map(|expires_at| expires_at > Instant::now())
            .unwrap_or(false)
}

fn summoner_ready_status_active(expires_at: Option<Instant>) -> bool {
    expires_at
        .map(|ready| ready > Instant::now())
        .unwrap_or(false)
}

fn summoner_can_use_solar_bahamut(level: u8) -> bool {
    level >= LEVEL_SUMMON_SOLAR_BAHAMUT
}

fn resolve_next_demi_state(smn: &SummonerState, level: u8) -> SummonerNextDemi {
    match smn.next_demi {
        SummonerNextDemi::None if summoner_can_use_solar_bahamut(level) => {
            SummonerNextDemi::SolarBahamutFirst
        }
        SummonerNextDemi::None => SummonerNextDemi::Bahamut,
        next if !summoner_can_use_solar_bahamut(level) => match next {
            SummonerNextDemi::SolarBahamutFirst | SummonerNextDemi::SolarBahamutSecond => {
                SummonerNextDemi::Bahamut
            }
            _ => next,
        },
        next => next,
    }
}

fn advance_summoner_next_demi_after_summon(
    next_demi: SummonerNextDemi,
    level: u8,
) -> SummonerNextDemi {
    if !summoner_can_use_solar_bahamut(level) {
        return SummonerNextDemi::Bahamut;
    }

    match next_demi {
        SummonerNextDemi::SolarBahamutFirst => SummonerNextDemi::Bahamut,
        SummonerNextDemi::SolarBahamutSecond => SummonerNextDemi::Bahamut,
        SummonerNextDemi::Bahamut | SummonerNextDemi::None => SummonerNextDemi::SolarBahamutSecond,
    }
}

fn summoner_has_any_runtime_timer(smn: &SummonerState) -> bool {
    smn.attunement_expires_at.is_some()
        || smn.demi_expires_at.is_some()
        || smn.primal_summon_expires_at.is_some()
        || smn.further_ruin_expires_at.is_some()
        || smn.searing_light_expires_at.is_some()
        || smn.searing_flash_expires_at.is_some()
        || smn.lux_solaris_expires_at.is_some()
}

fn clear_further_ruin(smn: &mut SummonerState) {
    smn.further_ruin = 0;
    smn.further_ruin_expires_at = None;
}

fn clear_attunement(smn: &mut SummonerState) {
    smn.attunement = SummonerAttunement::None;
    smn.attunement_stacks = 0;
    smn.attunement_expires_at = None;
}

fn clear_primal_summon_timer(smn: &mut SummonerState) {
    smn.primal_summon_expires_at = None;
}

fn clear_egi_ready_state(smn: &mut SummonerState) {
    smn.mountain_buster_ready = false;
    smn.slipstream_ready = false;
    smn.crimson_cyclone_ready = false;
    smn.crimson_strike_ready = false;
}

fn clear_expired_attunement_ready_state(smn: &mut SummonerState) {
    smn.mountain_buster_ready = false;
    smn.crimson_cyclone_ready = false;
    smn.crimson_strike_ready = false;
}

fn clear_demi_state(smn: &mut SummonerState) {
    smn.demi_phase = SummonerDemiPhase::None;
    smn.demi_expires_at = None;
    smn.demi_enkindle_ready = false;
    smn.demi_finisher_ready = false;
    smn.demi_auto_attack_count = 0;
}

fn refresh_summoner_runtime_state(smn: &mut SummonerState) {
    let now = Instant::now();

    if smn
        .attunement_expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        clear_attunement(smn);
        clear_expired_attunement_ready_state(smn);
    }

    if smn
        .demi_expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        clear_demi_state(smn);
    }

    if smn
        .primal_summon_expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        clear_primal_summon_timer(smn);
    }

    if smn
        .further_ruin_expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        clear_further_ruin(smn);
    }

    if smn
        .searing_light_expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        smn.searing_light_expires_at = None;
    }

    if smn
        .searing_flash_expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        smn.searing_flash_ready = false;
        smn.searing_flash_expires_at = None;
    }

    if smn
        .lux_solaris_expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        smn.lux_solaris_ready = false;
        smn.lux_solaris_expires_at = None;
    }
}

fn start_demi_phase(smn: &mut SummonerState, phase: SummonerDemiPhase) {
    let now = Instant::now();
    smn.demi_phase = phase;
    smn.demi_expires_at = Some(now + SUMMONER_DEMI_DURATION);
    smn.primal_summon_expires_at = None;
    smn.demi_enkindle_ready = true;
    smn.demi_finisher_ready = true;
    smn.demi_auto_attack_count = 0;
    smn.ruby_arcanum = true;
    smn.topaz_arcanum = true;
    smn.emerald_arcanum = true;
    clear_attunement(smn);
    clear_egi_ready_state(smn);
}

fn start_primal_summon_window(smn: &mut SummonerState) {
    smn.primal_summon_expires_at = Some(Instant::now() + SUMMONER_PRIMAL_WAIT_FOR_TARGET_DURATION);
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SummonerDemiAutoAttack {
    pub action_id: u32,
    pub pet_base_id: u32,
    pub potency: u32,
}

pub(crate) fn pending_demi_auto_attack(
    combat_state: &PlayerCombatState,
) -> Option<SummonerDemiAutoAttack> {
    let smn = &combat_state.summoner;
    if !summoner_demi_active(smn) || smn.demi_auto_attack_count >= SUMMONER_DEMI_AUTO_ATTACKS {
        return None;
    }

    match smn.demi_phase {
        SummonerDemiPhase::Bahamut => Some(SummonerDemiAutoAttack {
            action_id: ACTION_WYRM_WAVE,
            pet_base_id: SUMMONER_DEMI_BAHAMUT_BASE_ID,
            potency: SUMMONER_DEMI_BAHAMUT_AUTO_ATTACK_POTENCY,
        }),
        SummonerDemiPhase::SolarBahamut => Some(SummonerDemiAutoAttack {
            action_id: ACTION_SCARLET_FLAME,
            pet_base_id: SUMMONER_DEMI_SOLAR_BAHAMUT_BASE_ID,
            potency: SUMMONER_DEMI_SOLAR_BAHAMUT_AUTO_ATTACK_POTENCY,
        }),
        SummonerDemiPhase::None => None,
    }
}

pub(crate) fn mark_demi_auto_attack_used(combat_state: &mut PlayerCombatState) {
    combat_state.summoner.demi_auto_attack_count = combat_state
        .summoner
        .demi_auto_attack_count
        .saturating_add(1)
        .min(SUMMONER_DEMI_AUTO_ATTACKS);
}

fn summoner_attunement_available(smn: &SummonerState) -> bool {
    smn.attunement != SummonerAttunement::None
        && smn.attunement_stacks > 0
        && summoner_window_active(smn)
}

fn summoner_gauge_attunement_type(attunement: SummonerAttunement) -> u8 {
    match attunement {
        SummonerAttunement::None => 0,
        SummonerAttunement::Ruby => SUMMONER_GAUGE_ATTUNEMENT_RUBY,
        SummonerAttunement::Topaz => SUMMONER_GAUGE_ATTUNEMENT_TOPAZ,
        SummonerAttunement::Emerald => SUMMONER_GAUGE_ATTUNEMENT_EMERALD,
    }
}

pub(crate) fn build_summoner_gauge_data(combat_state: &PlayerCombatState, level: u8) -> u64 {
    let mut smn = combat_state.summoner.clone();
    refresh_summoner_runtime_state(&mut smn);
    let attunement_active = summoner_attunement_available(&smn);
    let next_demi = resolve_next_demi_state(&smn, level);

    let summon_expires_at = smn.demi_expires_at.or(smn.primal_summon_expires_at);
    let summon_timer = summon_expires_at
        .map(|expires_at| {
            expires_at
                .saturating_duration_since(Instant::now())
                .as_millis()
                .min(u128::from(u16::MAX)) as u16
        })
        .unwrap_or_default();

    let attunement_timer = smn
        .attunement_expires_at
        .map(|expires_at| {
            expires_at
                .saturating_duration_since(Instant::now())
                .as_millis()
                .min(u128::from(u16::MAX)) as u16
        })
        .unwrap_or_default();

    let attunement = if attunement_active {
        (smn.attunement_stacks << 2) | summoner_gauge_attunement_type(smn.attunement)
    } else {
        0
    };

    let mut aether_flags = match smn.aetherflow_stacks.min(2) {
        0 => 0,
        1 => SUMMONER_GAUGE_FLAG_AETHERFLOW_1,
        _ => SUMMONER_GAUGE_FLAG_AETHERFLOW_2,
    };
    if smn.ruby_arcanum {
        aether_flags |= SUMMONER_GAUGE_FLAG_IFRIT_READY;
    }
    if smn.topaz_arcanum {
        aether_flags |= SUMMONER_GAUGE_FLAG_TITAN_READY;
    }
    if smn.emerald_arcanum {
        aether_flags |= SUMMONER_GAUGE_FLAG_GARUDA_READY;
    }
    // Solar Bahamut indicator bit (bit 3, optionally with bit 2 for the 2nd cycle).
    // Retail behaviour, verified against the chronofoil capture of an SMN100 burst:
    //   * Currently in SolarBahamut demi phase → bit 3 set (regardless of 1st/2nd cycle).
    //     The client uses this to drive the demi countdown widget and the "in burst" UI;
    //     without it the SummonTimer is ignored and the demi visual never appears.
    //   * Currently in Bahamut demi phase → no demi indicator bits, even though
    //     `next_demi` may already be SolarBahamutSecond — only the *not in demi* gauge
    //     advertises the upcoming summon.
    //   * No active demi → fall back to `next_demi` to flag the next summon: SolarFirst
    //     gets bit 3, SolarSecond gets bits 2+3, Bahamut/None leaves them clear.
    match smn.demi_phase {
        SummonerDemiPhase::SolarBahamut => {
            aether_flags |= SUMMONER_GAUGE_FLAG_SOLAR_BAHAMUT_FIRST_PRIMED;
        }
        SummonerDemiPhase::Bahamut => {}
        SummonerDemiPhase::None => match next_demi {
            SummonerNextDemi::SolarBahamutFirst => {
                aether_flags |= SUMMONER_GAUGE_FLAG_SOLAR_BAHAMUT_FIRST_PRIMED;
            }
            SummonerNextDemi::SolarBahamutSecond => {
                aether_flags |= SUMMONER_GAUGE_FLAG_SOLAR_BAHAMUT_SECOND_PRIMED;
            }
            _ => {}
        },
    }

    let carbuncle: u8 = if smn.carbuncle_summoned {
        SUMMONER_GAUGE_CARBUNCLE
    } else {
        0
    };

    // Pack to match the client's SummonerGauge.Payload (little-endian u64):
    //   bytes 0-1 SummonTimer | 2-3 AttunementTimer | 4 ReturnSummon
    //   5 ReturnSummonGlam | 6 Attunement | 7 AetherFlags
    (summon_timer as u64)
        | ((attunement_timer as u64) << 16)
        | ((carbuncle as u64) << 32)
        | ((attunement as u64) << 48)
        | ((aether_flags as u64) << 56)
}

pub(crate) fn resolve_summoner_action(
    request: &ActionRequest,
    combat_state: &PlayerCombatState,
    level: u8,
    game_data: &mut GameData,
) -> u32 {
    let mut smn = combat_state.summoner.clone();
    refresh_summoner_runtime_state(&mut smn);
    let demi_active = summoner_demi_active(&smn);
    let next_demi = resolve_next_demi_state(&smn, level);

    match request.action_id {
        ACTION_SUMMON_BAHAMUT
            if smn.carbuncle_summoned
                && !demi_active
                && matches!(
                    next_demi,
                    SummonerNextDemi::SolarBahamutFirst | SummonerNextDemi::SolarBahamutSecond
                ) =>
        {
            ACTION_SUMMON_SOLAR_BAHAMUT
        }
        ACTION_RUIN if summoner_demi_active(&smn) => match smn.demi_phase {
            SummonerDemiPhase::Bahamut => ACTION_ASTRAL_IMPULSE,
            SummonerDemiPhase::SolarBahamut => ACTION_UMBRAL_IMPULSE,
            SummonerDemiPhase::None => ACTION_RUIN,
        },
        ACTION_RUIN if smn.further_ruin > 0 => ACTION_RUIN_IV,
        ACTION_RUIN_III if summoner_demi_active(&smn) => match smn.demi_phase {
            SummonerDemiPhase::Bahamut => ACTION_ASTRAL_IMPULSE,
            SummonerDemiPhase::SolarBahamut => ACTION_UMBRAL_IMPULSE,
            SummonerDemiPhase::None => ACTION_RUIN_III,
        },
        ACTION_OUTBURST if summoner_demi_active(&smn) => match smn.demi_phase {
            SummonerDemiPhase::Bahamut => ACTION_ASTRAL_FLARE,
            SummonerDemiPhase::SolarBahamut => ACTION_UMBRAL_FLARE,
            SummonerDemiPhase::None => ACTION_OUTBURST,
        },
        ACTION_OUTBURST if level >= game_data.get_trait_level(TRAIT_OUTBURST_MASTERY) => {
            ACTION_TRI_DISASTER
        }
        ACTION_GEMSHINE => match smn.attunement {
            SummonerAttunement::Ruby if summoner_attunement_available(&smn) => ACTION_RUBY_RUIN,
            SummonerAttunement::Topaz if summoner_attunement_available(&smn) => ACTION_TOPAZ_RUIN,
            SummonerAttunement::Emerald if summoner_attunement_available(&smn) => {
                ACTION_EMERALD_RUIN
            }
            _ => ACTION_GEMSHINE,
        },
        ACTION_PRECIOUS_BRILLIANCE => match smn.attunement {
            SummonerAttunement::Ruby if summoner_attunement_available(&smn) => ACTION_RUBY_DISASTER,
            SummonerAttunement::Topaz if summoner_attunement_available(&smn) => {
                ACTION_TOPAZ_DISASTER
            }
            SummonerAttunement::Emerald if summoner_attunement_available(&smn) => {
                ACTION_EMERALD_DISASTER
            }
            _ => ACTION_PRECIOUS_BRILLIANCE,
        },
        ACTION_FESTER if smn.aetherflow_stacks > 0 => ACTION_NECROTIZE,
        ACTION_ASTRAL_FLOW => {
            if smn.crimson_strike_ready && summoner_window_active(&smn) {
                ACTION_CRIMSON_STRIKE
            } else if smn.crimson_cyclone_ready && summoner_window_active(&smn) {
                ACTION_CRIMSON_CYCLONE
            } else if demi_active
                && smn.demi_phase == SummonerDemiPhase::Bahamut
                && smn.demi_finisher_ready
            {
                ACTION_DEATHFLARE
            } else if demi_active
                && smn.demi_phase == SummonerDemiPhase::SolarBahamut
                && smn.demi_finisher_ready
            {
                ACTION_SUNFLARE
            } else if smn.mountain_buster_ready && summoner_window_active(&smn) {
                ACTION_MOUNTAIN_BUSTER
            } else if smn.slipstream_ready {
                ACTION_SLIPSTREAM
            } else {
                ACTION_ASTRAL_FLOW
            }
        }
        ACTION_ENKINDLE_BAHAMUT
            if demi_active && smn.demi_phase == SummonerDemiPhase::SolarBahamut =>
        {
            ACTION_ENKINDLE_SOLAR_BAHAMUT
        }
        ACTION_RUIN | ACTION_RUIN_III | ACTION_OUTBURST => request.action_id,
        _ => request.action_id,
    }
}

pub(crate) fn can_execute_summoner_action(
    action_id: u32,
    combat_state: &PlayerCombatState,
    level: u8,
) -> bool {
    let mut smn = combat_state.summoner.clone();
    refresh_summoner_runtime_state(&mut smn);
    let demi_active = summoner_demi_active(&smn);
    let next_demi = resolve_next_demi_state(&smn, level);

    match action_id {
        ACTION_SUMMON_CARBUNCLE => !demi_active,
        ACTION_AETHERCHARGE => {
            smn.carbuncle_summoned
                && !smn.ruby_arcanum
                && !smn.topaz_arcanum
                && !smn.emerald_arcanum
                && !demi_active
                && !summoner_attunement_available(&smn)
        }
        ACTION_SUMMON_BAHAMUT | ACTION_SUMMON_SOLAR_BAHAMUT => {
            // Retail allows a new demi summon to override unspent elemental arcanum/attunement
            // once the demi button is ready. The next `start_demi_phase` call resets those states.
            let summon_ready = smn.carbuncle_summoned && !demi_active;

            match action_id {
                ACTION_SUMMON_SOLAR_BAHAMUT => {
                    summon_ready
                        && summoner_can_use_solar_bahamut(level)
                        && matches!(
                            next_demi,
                            SummonerNextDemi::SolarBahamutFirst
                                | SummonerNextDemi::SolarBahamutSecond
                        )
                }
                ACTION_SUMMON_BAHAMUT => {
                    summon_ready && matches!(next_demi, SummonerNextDemi::Bahamut)
                }
                _ => false,
            }
        }
        ACTION_SUMMON_RUBY | ACTION_SUMMON_IFRIT_II => {
            smn.carbuncle_summoned && smn.ruby_arcanum && !demi_active
        }
        ACTION_SUMMON_TOPAZ | ACTION_SUMMON_TITAN_II => {
            smn.carbuncle_summoned && smn.topaz_arcanum && !demi_active
        }
        ACTION_SUMMON_EMERALD | ACTION_SUMMON_GARUDA_II => {
            smn.carbuncle_summoned && smn.emerald_arcanum && !demi_active
        }
        ACTION_RUIN_IV => smn.further_ruin > 0,
        ACTION_FESTER | ACTION_PAINFLARE | ACTION_NECROTIZE => smn.aetherflow_stacks > 0,
        ACTION_ASTRAL_IMPULSE => demi_active && smn.demi_phase == SummonerDemiPhase::Bahamut,
        ACTION_ASTRAL_FLARE => demi_active && smn.demi_phase == SummonerDemiPhase::Bahamut,
        ACTION_UMBRAL_IMPULSE => demi_active && smn.demi_phase == SummonerDemiPhase::SolarBahamut,
        ACTION_UMBRAL_FLARE => demi_active && smn.demi_phase == SummonerDemiPhase::SolarBahamut,
        ACTION_ENKINDLE_BAHAMUT => {
            demi_active && smn.demi_phase == SummonerDemiPhase::Bahamut && smn.demi_enkindle_ready
        }
        ACTION_ENKINDLE_SOLAR_BAHAMUT => {
            demi_active
                && smn.demi_phase == SummonerDemiPhase::SolarBahamut
                && smn.demi_enkindle_ready
        }
        ACTION_DEATHFLARE => {
            demi_active && smn.demi_phase == SummonerDemiPhase::Bahamut && smn.demi_finisher_ready
        }
        ACTION_SUNFLARE => {
            demi_active
                && smn.demi_phase == SummonerDemiPhase::SolarBahamut
                && smn.demi_finisher_ready
        }
        ACTION_LUX_SOLARIS => {
            smn.lux_solaris_ready && summoner_ready_status_active(smn.lux_solaris_expires_at)
        }
        ACTION_SEARING_FLASH => {
            smn.searing_flash_ready && summoner_ready_status_active(smn.searing_flash_expires_at)
        }
        ACTION_RUBY_RUIN | ACTION_RUBY_DISASTER => {
            smn.attunement == SummonerAttunement::Ruby && summoner_attunement_available(&smn)
        }
        ACTION_TOPAZ_RUIN | ACTION_TOPAZ_DISASTER => {
            smn.attunement == SummonerAttunement::Topaz && summoner_attunement_available(&smn)
        }
        ACTION_EMERALD_RUIN | ACTION_EMERALD_DISASTER => {
            smn.attunement == SummonerAttunement::Emerald && summoner_attunement_available(&smn)
        }
        ACTION_ASTRAL_FLOW => {
            summoner_window_active(&smn)
                && (smn.mountain_buster_ready
                    || smn.slipstream_ready
                    || smn.crimson_cyclone_ready
                    || smn.crimson_strike_ready)
        }
        ACTION_MOUNTAIN_BUSTER => smn.mountain_buster_ready && summoner_window_active(&smn),
        ACTION_SLIPSTREAM => smn.slipstream_ready,
        ACTION_CRIMSON_CYCLONE => smn.crimson_cyclone_ready && summoner_window_active(&smn),
        ACTION_CRIMSON_STRIKE => smn.crimson_strike_ready && summoner_window_active(&smn),
        _ => true,
    }
}

fn refresh_summoner_statuses(actor: &mut NetworkedActor, owner_actor_id: ObjectId) {
    let NetworkedActor::Player {
        combat_state,
        status_effects,
        ..
    } = actor
    else {
        return;
    };

    refresh_summoner_runtime_state(&mut combat_state.summoner);

    status_effects.remove(STATUS_FURTHER_RUIN);
    status_effects.remove(STATUS_DREADWYRM_TRANCE);
    status_effects.remove(STATUS_SEARING_FLASH_READY);
    status_effects.remove(STATUS_LUX_SOLARIS_READY);
    status_effects.remove(STATUS_CRIMSON_CYCLONE_READY);
    status_effects.remove(STATUS_SLIPSTREAM_READY);
    status_effects.remove(STATUS_MOUNTAIN_BUSTER_READY);
    status_effects.remove(STATUS_GARUDA_ATTUNEMENT);
    status_effects.remove(STATUS_TITAN_ATTUNEMENT);
    status_effects.remove(STATUS_IFRIT_ATTUNEMENT);
    status_effects.remove(STATUS_CRIMSON_STRIKE_READY);

    let smn = &combat_state.summoner;
    if smn.further_ruin > 0 {
        let remaining = smn
            .further_ruin_expires_at
            .map(|expires_at| {
                expires_at
                    .saturating_duration_since(Instant::now())
                    .as_secs_f32()
            })
            .unwrap_or(0.0);
        status_effects.add(STATUS_FURTHER_RUIN, 0, remaining);
    }
    if let Some(expires_at) = smn.searing_light_expires_at {
        let remaining = expires_at
            .saturating_duration_since(Instant::now())
            .as_secs_f32();
        status_effects.add(STATUS_SEARING_LIGHT, 0, remaining);
    }
    let self_source = owner_actor_id;

    if smn.searing_flash_ready {
        let remaining = smn
            .searing_flash_expires_at
            .map(|expires_at| {
                expires_at
                    .saturating_duration_since(Instant::now())
                    .as_secs_f32()
            })
            .unwrap_or(0.0);
        status_effects.add_with_source(STATUS_SEARING_FLASH_READY, 0, remaining, self_source);
    }
    if smn.lux_solaris_ready {
        let remaining = smn
            .lux_solaris_expires_at
            .map(|expires_at| {
                expires_at
                    .saturating_duration_since(Instant::now())
                    .as_secs_f32()
            })
            .unwrap_or(0.0);
        status_effects.add_with_source(STATUS_LUX_SOLARIS_READY, 0, remaining, self_source);
    }
    if smn.demi_phase == SummonerDemiPhase::Bahamut && summoner_demi_active(smn) {
        let remaining = smn
            .demi_expires_at
            .map(|expires_at| {
                expires_at
                    .saturating_duration_since(Instant::now())
                    .as_secs_f32()
            })
            .unwrap_or(0.0);
        status_effects.add_with_source(STATUS_DREADWYRM_TRANCE, 0, remaining, self_source);
    }
    // The egi-assault "ready" procs have no timer on retail (duration 0.000 = last until consumed);
    // the state machine removes them when used. A nonzero duration would show a bogus countdown.
    if smn.crimson_cyclone_ready {
        status_effects.add_with_source(STATUS_CRIMSON_CYCLONE_READY, 0, 0.0, self_source);
    }
    if smn.slipstream_ready {
        status_effects.add_with_source(STATUS_SLIPSTREAM_READY, 0, 0.0, self_source);
    }
    if smn.mountain_buster_ready {
        status_effects.add_with_source(STATUS_MOUNTAIN_BUSTER_READY, 0, 0.0, self_source);
    }
    if smn.crimson_strike_ready {
        status_effects.add_with_source(STATUS_CRIMSON_STRIKE_READY, 0, 0.0, self_source);
    }

    let attunement_status = match smn.attunement {
        SummonerAttunement::Ruby => Some(STATUS_IFRIT_ATTUNEMENT),
        SummonerAttunement::Topaz => Some(STATUS_TITAN_ATTUNEMENT),
        SummonerAttunement::Emerald => Some(STATUS_GARUDA_ATTUNEMENT),
        SummonerAttunement::None => None,
    };

    if let Some(status_id) = attunement_status {
        // Show the real remaining attunement time so the buff timer counts down correctly.
        let remaining = smn
            .attunement_expires_at
            .map(|expires_at| {
                expires_at
                    .saturating_duration_since(Instant::now())
                    .as_secs_f32()
            })
            .unwrap_or(0.0);
        status_effects.add_with_source(
            status_id,
            u16::from(smn.attunement_stacks),
            remaining,
            self_source,
        );
    }
}

pub(crate) fn update_summoner_state_after_action(
    action_id: u32,
    actor: &mut NetworkedActor,
    owner_actor_id: ObjectId,
) {
    let level = actor.get_common_spawn().level;
    let NetworkedActor::Player { combat_state, .. } = actor else {
        return;
    };

    let smn = &mut combat_state.summoner;
    refresh_summoner_runtime_state(smn);
    smn.next_demi = resolve_next_demi_state(smn, level);

    match action_id {
        ACTION_SUMMON_CARBUNCLE => {
            smn.carbuncle_summoned = true;
            clear_egi_ready_state(smn);
            smn.next_demi = resolve_next_demi_state(smn, level);
        }
        ACTION_AETHERCHARGE => {
            smn.ruby_arcanum = true;
            smn.topaz_arcanum = true;
            smn.emerald_arcanum = true;
            clear_attunement(smn);
            clear_egi_ready_state(smn);
        }
        ACTION_SEARING_LIGHT => {
            smn.searing_light_expires_at = Some(Instant::now() + SUMMONER_SEARING_LIGHT_DURATION);
            smn.searing_flash_ready = true;
            smn.searing_flash_expires_at = Some(Instant::now() + SUMMONER_READY_STATUS_DURATION);
        }
        ACTION_SUMMON_SOLAR_BAHAMUT => {
            start_demi_phase(smn, SummonerDemiPhase::SolarBahamut);
            smn.lux_solaris_ready = true;
            smn.lux_solaris_expires_at = Some(Instant::now() + SUMMONER_READY_STATUS_DURATION);
            smn.next_demi = advance_summoner_next_demi_after_summon(smn.next_demi, level);
        }
        ACTION_SUMMON_BAHAMUT => {
            start_demi_phase(smn, SummonerDemiPhase::Bahamut);
            smn.next_demi = advance_summoner_next_demi_after_summon(smn.next_demi, level);
        }
        ACTION_SUMMON_RUBY | ACTION_SUMMON_IFRIT_II => {
            smn.ruby_arcanum = false;
            smn.attunement = SummonerAttunement::Ruby;
            smn.attunement_stacks = 2;
            smn.attunement_expires_at = Some(Instant::now() + SUMMONER_ATTUNEMENT_DURATION);
            start_primal_summon_window(smn);
            smn.mountain_buster_ready = false;
            smn.slipstream_ready = false;
            smn.crimson_cyclone_ready = true;
            smn.crimson_strike_ready = false;
        }
        ACTION_SUMMON_TOPAZ | ACTION_SUMMON_TITAN_II => {
            smn.topaz_arcanum = false;
            smn.attunement = SummonerAttunement::Topaz;
            smn.attunement_stacks = 4;
            smn.attunement_expires_at = Some(Instant::now() + SUMMONER_ATTUNEMENT_DURATION);
            start_primal_summon_window(smn);
            smn.mountain_buster_ready = false;
            smn.slipstream_ready = false;
            smn.crimson_cyclone_ready = false;
            smn.crimson_strike_ready = false;
        }
        ACTION_SUMMON_EMERALD | ACTION_SUMMON_GARUDA_II => {
            smn.emerald_arcanum = false;
            smn.attunement = SummonerAttunement::Emerald;
            smn.attunement_stacks = 4;
            smn.attunement_expires_at = Some(Instant::now() + SUMMONER_ATTUNEMENT_DURATION);
            start_primal_summon_window(smn);
            smn.mountain_buster_ready = false;
            smn.slipstream_ready = true;
            smn.crimson_cyclone_ready = false;
            smn.crimson_strike_ready = false;
        }
        ACTION_ENERGY_DRAIN | ACTION_ENERGY_SIPHON => {
            smn.aetherflow_stacks = 2;
            smn.further_ruin = 1;
            smn.further_ruin_expires_at = Some(Instant::now() + SUMMONER_FURTHER_RUIN_DURATION);
        }
        ACTION_ENKINDLE_BAHAMUT | ACTION_ENKINDLE_SOLAR_BAHAMUT => {
            smn.demi_enkindle_ready = false;
        }
        ACTION_DEATHFLARE | ACTION_SUNFLARE => {
            smn.demi_finisher_ready = false;
        }
        ACTION_LUX_SOLARIS => {
            smn.lux_solaris_ready = false;
            smn.lux_solaris_expires_at = None;
        }
        ACTION_SEARING_FLASH => {
            smn.searing_flash_ready = false;
            smn.searing_flash_expires_at = None;
        }
        ACTION_FESTER | ACTION_PAINFLARE => {
            smn.aetherflow_stacks = smn.aetherflow_stacks.saturating_sub(1);
        }
        ACTION_RUIN_IV => {
            smn.further_ruin = smn.further_ruin.saturating_sub(1);
            if smn.further_ruin == 0 {
                smn.further_ruin_expires_at = None;
            }
        }
        ACTION_RUBY_RUIN | ACTION_RUBY_DISASTER => {
            smn.attunement_stacks = smn.attunement_stacks.saturating_sub(1);
            if smn.attunement_stacks == 0 {
                smn.attunement = SummonerAttunement::None;
                smn.attunement_expires_at = None;
            }
        }
        ACTION_TOPAZ_RUIN | ACTION_TOPAZ_DISASTER => {
            smn.attunement_stacks = smn.attunement_stacks.saturating_sub(1);
            smn.mountain_buster_ready = true;
            if smn.attunement_stacks == 0 {
                smn.attunement = SummonerAttunement::None;
                smn.attunement_expires_at = None;
            }
        }
        ACTION_EMERALD_RUIN | ACTION_EMERALD_DISASTER => {
            smn.attunement_stacks = smn.attunement_stacks.saturating_sub(1);
            if smn.attunement_stacks == 0 {
                smn.attunement = SummonerAttunement::None;
                smn.attunement_expires_at = None;
            }
        }
        ACTION_MOUNTAIN_BUSTER => {
            smn.mountain_buster_ready = false;
        }
        ACTION_SLIPSTREAM => {
            smn.slipstream_ready = false;
        }
        ACTION_CRIMSON_CYCLONE => {
            smn.crimson_cyclone_ready = false;
            smn.crimson_strike_ready = true;
        }
        ACTION_CRIMSON_STRIKE => {
            smn.crimson_strike_ready = false;
        }
        _ => {}
    }

    refresh_summoner_statuses(actor, owner_actor_id);
}

/// Result of [`refresh_summoner_runtime_state_on_actor`]: whether the cloned state actually
/// changed (drives gauge resends and status icon refreshes), and whether the demi window just
/// expired this tick (drives the demi-revert ActorControlSelf packets).
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct SummonerRefreshResult {
    pub changed: bool,
    pub demi_just_ended: bool,
    pub status_timer_refreshed: bool,
}

pub(crate) fn refresh_summoner_runtime_state_on_actor(
    owner_actor_id: ObjectId,
    actor: &mut NetworkedActor,
) -> SummonerRefreshResult {
    let level = actor.get_common_spawn().level;
    let NetworkedActor::Player {
        combat_state,
        status_effects,
        ..
    } = actor
    else {
        return SummonerRefreshResult::default();
    };

    let had_runtime_timer = summoner_has_any_runtime_timer(&combat_state.summoner);
    let before = combat_state.summoner.clone();
    refresh_summoner_runtime_state(&mut combat_state.summoner);
    combat_state.summoner.next_demi = resolve_next_demi_state(&combat_state.summoner, level);
    let changed = combat_state.summoner != before;
    // Demi just expired this refresh: the demi phase was non-None before, and refresh cleared it
    // (which can only happen when `demi_expires_at <= now`). Consumers use this edge to send the
    // ActorControlSelf packets that tear down the demi UI on the client.
    let demi_just_ended = before.demi_phase != SummonerDemiPhase::None
        && combat_state.summoner.demi_phase == SummonerDemiPhase::None;

    let status_timer_refreshed = changed || had_runtime_timer;
    if status_timer_refreshed {
        let _ = status_effects;
        refresh_summoner_statuses(actor, owner_actor_id);
    }

    SummonerRefreshResult {
        changed,
        demi_just_ended,
        status_timer_refreshed,
    }
}

/// Apply a single gauge action (from `EffectsBuilder:modify_gauge`) to the player's combat state.
pub(crate) fn apply_gauge_action(combat_state: &mut PlayerCombatState, action: &GaugeAction) {
    match action.index {
        GAUGE_INDEX_AETHERFLOW => {
            let stacks = combat_state.summoner.aetherflow_stacks as i32 + action.amount;
            combat_state.summoner.aetherflow_stacks = stacks.clamp(0, MAX_AETHERFLOW as i32) as u8;
        }
        other => tracing::warn!("modify_gauge: unknown gauge index {other}"),
    }
}
