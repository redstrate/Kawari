use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use glam::Vec3;
use kawari::{
    common::{
        DirectorEvent, EOBJ_EXIT, EOBJ_SHORTCUT, EventState, HandlerId, JumpState,
        MoveAnimationState, MoveAnimationType, ObjectId, ObjectTypeId, ObjectTypeKind, Position,
    },
    ipc::zone::{
        ActionEffect, ActionType, ActorControlCategory, ActorControlSelf, AoeEffect8,
        AoeEffectHeader, BattleNpcSubKind, CharacterDataFlag, CommonSpawn, DamageElement,
        DamageKind, DamageType, DisplayFlag, EffectKind, ObjectKind,
        ServerZoneIpcData, ServerZoneIpcSegment, SpawnNpc,
    },
};
use mlua::{Function, Lua, LuaSerdeExt, RegistryKey, Table, UserData, UserDataMethods, Value};
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, GameData, ToServer,
    lua::KawariLua,
    server::{
        WorldServer,
        action::update_actor_hp_mp,
        actor::{NetworkedActor, NpcState},
        effect::gain_effect_instance,
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
    },
};

/// A value stored in encounter-scoped state (global vars or per-player vars). Mirrors the Lua
/// value types a mechanic script needs to stash across phases.
#[derive(Debug, Clone, PartialEq)]
pub enum EncounterVar {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

impl EncounterVar {
    /// Convert a Lua value into an `EncounterVar`. Returns None for nil/unsupported types (the
    /// caller treats that as "clear this var"). Actor ids are passed as plain integers.
    fn from_lua(value: &Value) -> Option<Self> {
        match value {
            Value::Boolean(b) => Some(Self::Bool(*b)),
            Value::Integer(i) => Some(Self::Int(*i)),
            Value::Number(n) => Some(Self::Float(*n)),
            Value::String(s) => s.to_str().ok().map(|s| Self::Str(s.to_string())),
            _ => None,
        }
    }

    fn to_lua(&self, lua: &Lua) -> mlua::Result<Value> {
        Ok(match self {
            Self::Bool(b) => Value::Boolean(*b),
            Self::Int(i) => Value::Integer(*i),
            Self::Float(f) => Value::Number(*f),
            Self::Str(s) => Value::String(lua.create_string(s)?),
        })
    }
}

/// A lightweight, read-only snapshot of an actor taken at the start of an encounter tick. Lua
/// mechanic callbacks query these (position, hp, alive) instead of touching the live instance, so
/// the high-frequency encounter tick can stay pure-`data` and the Lua director stays detached.
#[derive(Debug, Clone, Copy)]
pub struct ActorSnapshot {
    pub id: ObjectId,
    pub is_player: bool,
    /// True if this actor is one of the registered bosses.
    pub is_boss: bool,
    pub position: Position,
    pub rotation: f32,
    pub hp: u32,
    pub max_hp: u32,
}

impl ActorSnapshot {
    fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// A server-side AoE shape used for hit detection. All checks are done in the X-Z plane (Y is
/// ignored, matching how retail resolves ground AoEs). `rotation` is the FFXIV yaw of the shape,
/// where the forward direction is `(sin θ, cos θ)` in X-Z — the same convention the pet/summon code
/// uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AoeShape {
    /// A filled circle of `radius` around the origin.
    Circle { radius: f32 },
    /// A ring: hits between `inner` and `outer` radius (a donut / "get in" mechanic).
    Donut { inner: f32, outer: f32 },
    /// A forward rectangle (line AoE): `length` forward from the origin, `width` total side-to-side.
    Rect { length: f32, width: f32 },
    /// A forward cone of `radius` with a total opening of `angle` degrees, centered on facing.
    Cone { radius: f32, angle: f32 },
    /// Hits every (alive) player regardless of position — a raidwide.
    Everyone,
}

impl AoeShape {
    /// Returns true if `point` lies inside this shape positioned at `origin` facing `rotation`.
    pub fn contains(&self, point: Vec3, origin: Vec3, rotation: f32) -> bool {
        // Work strictly in the horizontal plane.
        let dx = point.x - origin.x;
        let dz = point.z - origin.z;
        let dist_sq = dx * dx + dz * dz;

        match *self {
            AoeShape::Everyone => true,
            AoeShape::Circle { radius } => dist_sq <= radius * radius,
            AoeShape::Donut { inner, outer } => {
                dist_sq >= inner * inner && dist_sq <= outer * outer
            }
            AoeShape::Rect { length, width } => {
                // Forward and right basis vectors in X-Z.
                let (fx, fz) = (rotation.sin(), rotation.cos());
                let forward = dx * fx + dz * fz; // distance ahead of origin
                let lateral = dx * fz - dz * fx; // signed side offset
                forward >= 0.0 && forward <= length && lateral.abs() <= width / 2.0
            }
            AoeShape::Cone { radius, angle } => {
                if dist_sq > radius * radius {
                    return false;
                }
                if dist_sq == 0.0 {
                    return true;
                }
                let (fx, fz) = (rotation.sin(), rotation.cos());
                let len = dist_sq.sqrt();
                // cos of the angle between the facing and the point.
                let cos = (dx * fx + dz * fz) / len;
                let half = (angle.to_radians() / 2.0).cos();
                cos >= half
            }
        }
    }

}

/// A pending AoE produced by a Lua `aoe_*`/`raidwide` call. The origin/rotation are captured at the
/// moment the Lua script schedules it (the telegraph location); only player *positions* are read at
/// resolution time, so dodging out resolves fairly. Resolved by a precise tokio timer, not the 8Hz
/// tick, so the snapshot lands at the exact activation time.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingAoe {
    pub shape: AoeShape,
    pub origin: Position,
    pub rotation: f32,
    /// Seconds from now until the AoE resolves.
    pub delay: f32,
    /// Action id for the result packet / animation (0 = none).
    pub action_id: u32,
    /// Flat damage dealt to each player caught in the shape.
    pub damage: u32,
    /// The actor the AoE originates from (boss/helper); also the enmity target.
    pub source_id: ObjectId,
    /// If set, the actor the result/effect packet is *animated* from (an off-arena omen helper), so
    /// the hit VFX doesn't burst at the boss and charge actions don't drag the boss. Enmity still
    /// goes to `source_id`. `None` → animate from `source_id` (legacy boss-sourced behaviour).
    pub effect_source: Option<ObjectId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LuaDirectorTask {
    HideEObj {
        base_id: u32,
    },
    ShowEObj {
        base_id: u32,
    },
    DeleteEObj {
        base_id: u32,
    },
    SpawnEObj {
        base_id: u32,
        position: Option<Position>,
    },
    SendVariables,
    AbandonDuty {
        actor_id: ObjectId,
    },
    BeginEventAction {
        actor_id: ObjectId,
        target: ObjectId,
        action_id: u32,
    },
    FinishGimmickEvent {
        actor_id: ObjectId,
    },
    LogMessage {
        id: u32,
        params: Vec<u32>,
    },
    SpawnBattleNpc {
        id: u32,
    },
    GainEffect {
        actor_id: ObjectId,
        id: u16,
        param: u16,
        duration: f32,
    },
    SetBGM {
        id: u32,
    },
    SealBossWall {
        actor_id: ObjectId,
        id: u32,
        place_name: u32,
        time_until: u32,
    },
    SpawnBoss {
        bnpc_id: u32,
        wall_id: u32,
        line_id: u32,
        place_name: u32,
    },
    /// Spawn a boss directly from a `BNpcBase` id (no dropin/LGB template needed), e.g. for trials
    /// like Ifrit that have no battle-npc dropin. Registered as a director boss so it anchors the
    /// encounter timeline. The spawned actor's `layout_id` is set to `base_id` so `onActorDeath`
    /// can identify it.
    SpawnBossByBase {
        base_id: u32,
        name_id: u32,
        hp: u32,
        level: u8,
        position: Position,
        rotation: f32,
    },
    SpawnTreasure {
        id: u32,
    },
    VariantVoteRoute {
        npc_route: u32,
    },
    PlayCutscene {
        cutscene_id: u32,
    },
    UpdateShortcut {
        poprange_id: u32,
    },
    UseShortcut {
        actor_id: ObjectId,
    },
    CompleteDuty {},
    MapEffect {
        index: u32,
        timeline_id: u32,
    },
    /// Spawn a named helper actor (cast/AoE source, telegraph carrier) from a base BNpc id.
    SpawnHelper {
        actor_id: ObjectId,
        bnpc_id: u32,
        position: Position,
        rotation: f32,
    },
    /// Despawn an actor (helper, add, etc.) by id.
    DespawnActor {
        actor_id: ObjectId,
    },
    /// Show an enemy cast bar from `source` for `action_id` lasting `cast_time` seconds.
    CastBar {
        source_id: ObjectId,
        action_id: u32,
        cast_time: f32,
        target_id: ObjectId,
    },
    /// Resolve an AoE after a precise delay. Collected by `director_tick` and handed back to the
    /// caller (which owns the `data`/`network` handles) so it can spawn the precise timer.
    ResolveAoe(PendingAoe),
    /// Show a ground telegraph (omen) by having `caster_id` cast `action_id` for `cast_time` seconds.
    /// `self_targeted` picks how the client anchors the omen: directional shapes (rect/cone) cast on
    /// the caster (target = self) and extend forward from `position`+`rotation`; location shapes
    /// (circles) cast no-target with the omen drawn at `position`.
    OmenCast {
        caster_id: ObjectId,
        action_id: u32,
        cast_time: f32,
        position: Position,
        rotation: f32,
        self_targeted: bool,
    },
    /// Spawn one invisible, non-hostile helper actor (parked at `position`) and add it to the omen
    /// caster pool. Used as an off-arena cast source for omens/AoE effects.
    SpawnOmenHelper {
        base_id: u32,
        name_id: u32,
        position: Position,
    },
    /// Send an `ActorControl` for `actor_id` to nearby clients (e.g. hide/show or (un)targetable a
    /// boss while it's off doing a mechanic).
    SendActorControl {
        actor_id: ObjectId,
        category: ActorControlCategory,
    },
    /// Pause/resume an NPC's behavior (chase + auto-attack). Pausing also clears its chase target so
    /// it stops moving; resuming re-acquires from its (kept) hate list.
    SetAiPaused {
        actor_id: ObjectId,
        paused: bool,
    },
    /// Teleport an actor to `position`/`rotation` (server state + ActorMove to clients).
    MoveActor {
        actor_id: ObjectId,
        position: Position,
        rotation: f32,
    },
    /// Spawn one visible clone actor (BNpcBase `base_id`, hidden until shown) and add it to the clone
    /// pool. Used for mechanics with visible duplicate bosses (e.g. Crimson Cyclone chargers).
    SpawnClone {
        base_id: u32,
        name_id: u32,
        position: Position,
        rotation: f32,
    },
}

// TODO: Maybe collapse into DirectorData?
#[derive(Default, Debug)]
pub struct LuaDirector {
    pub data: [u8; 10],
    pub tasks: Vec<LuaDirectorTask>,
    pub bosses: HashMap<u32, DirectorBoss>,
    /// Battle elapsed seconds at the time this director was invoked, so `schedule`/`every`
    /// can resolve relative times into absolute battle elapsed seconds.
    pub elapsed_secs: f64,
    /// Schedule requests made by the Lua script this invocation, merged into
    /// `DirectorData.scheduler` by `apply_lua_director`.
    pub pending_schedule: Vec<PendingSchedule>,
    /// Encounter-scoped global vars (a working copy synced back to `DirectorData` after the call).
    pub vars: HashMap<String, EncounterVar>,
    /// Encounter-scoped per-player vars (working copy synced back after the call).
    pub player_vars: HashMap<ObjectId, HashMap<String, EncounterVar>>,
    /// Read-only snapshot of the instance's actors this tick, for position/hp queries.
    pub actors: Vec<ActorSnapshot>,
    /// Named helper actor registry (name -> actor id). Two-way synced with `DirectorData`.
    pub helpers: HashMap<String, ObjectId>,
    /// Pool of invisible helper actors used as omen/AoE casters (so the boss isn't locked casting).
    pub omen_helpers: Vec<ObjectId>,
    /// Round-robin cursor into `omen_helpers`; synced back so consecutive omens use different ones.
    pub omen_rr: usize,
    /// Pool of visible clone actors (e.g. Crimson Cyclone chargers), looked up by `clone(index)`.
    pub clones: Vec<ObjectId>,
}

/// A `director:at/schedule/every` request produced inside a Lua callback. The callback function
/// is already stored in the Lua registry; `callback` is its key.
#[derive(Debug)]
pub struct PendingSchedule {
    pub at_secs: f64,
    pub callback: RegistryKey,
    pub repeat: Option<RepeatSpec>,
}

/// Repetition state for `director:every`.
#[derive(Debug, Clone, Copy)]
pub struct RepeatSpec {
    /// Seconds between repeats.
    pub interval: f64,
    /// How many fires are left (including the upcoming one).
    pub remaining: u32,
    /// Zero-based index passed to the Lua callback.
    pub index: u32,
}

/// A scheduled timeline event, fired when `at_secs <= elapsed`. Sorted ascending by `at_secs`.
#[derive(Debug)]
pub struct ScheduledEvent {
    pub at_secs: f64,
    pub callback: RegistryKey,
    pub repeat: Option<RepeatSpec>,
}

impl UserData for LuaDirector {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("hide_eobj", |_, this, base_id: u32| {
            this.tasks.push(LuaDirectorTask::HideEObj { base_id });
            Ok(())
        });
        methods.add_method_mut("show_eobj", |_, this, base_id: u32| {
            this.tasks.push(LuaDirectorTask::ShowEObj { base_id });
            Ok(())
        });
        methods.add_method_mut("delete_eobj", |_, this, base_id: u32| {
            this.tasks.push(LuaDirectorTask::DeleteEObj { base_id });
            Ok(())
        });
        methods.add_method_mut(
            "spawn_eobj",
            |lua, this, (base_id, position): (u32, Value)| {
                let position: Option<Position> = lua.from_value(position).ok();
                this.tasks
                    .push(LuaDirectorTask::SpawnEObj { base_id, position });
                Ok(())
            },
        );
        methods.add_method_mut("set_data", |_, this, (index, data): (u8, u8)| {
            this.data[index as usize] = data;
            Ok(())
        });
        methods.add_method("data", |_, this, index: u8| Ok(this.data[index as usize]));
        methods.add_method_mut("abandon_duty", |_, this, actor_id: u32| {
            this.tasks.push(LuaDirectorTask::AbandonDuty {
                actor_id: ObjectId(actor_id),
            });
            Ok(())
        });
        methods.add_method_mut(
            "event_action",
            |_, this, (action_id, actor_id, target): (u32, u32, u32)| {
                this.tasks.push(LuaDirectorTask::BeginEventAction {
                    actor_id: ObjectId(actor_id),
                    target: ObjectId(target),
                    action_id,
                });
                Ok(())
            },
        );
        methods.add_method_mut("finish_gimmick", |_, this, actor_id: u32| {
            this.tasks.push(LuaDirectorTask::FinishGimmickEvent {
                actor_id: ObjectId(actor_id),
            });
            Ok(())
        });
        methods.add_method_mut("log_message", |_, this, (id, params): (u32, Vec<u32>)| {
            this.tasks.push(LuaDirectorTask::LogMessage { id, params });
            Ok(())
        });
        methods.add_method_mut("spawn_bnpc", |_, this, id: u32| {
            this.tasks.push(LuaDirectorTask::SpawnBattleNpc { id });
            Ok(())
        });
        methods.add_method_mut(
            "gain_effect",
            |_, this, (actor_id, id, param, duration): (u32, u16, u16, f32)| {
                this.tasks.push(LuaDirectorTask::GainEffect {
                    actor_id: ObjectId(actor_id),
                    id,
                    param,
                    duration,
                });
                Ok(())
            },
        );
        methods.add_method_mut("set_bgm", |_, this, id: u32| {
            this.tasks.push(LuaDirectorTask::SetBGM { id });
            Ok(())
        });
        methods.add_method_mut(
            "spawn_boss",
            |_, this, (bnpc_id, wall_id, line_id, place_name): (u32, u32, u32, u32)| {
                this.tasks.push(LuaDirectorTask::SpawnBoss {
                    bnpc_id,
                    wall_id,
                    line_id,
                    place_name,
                });
                Ok(())
            },
        );
        // Spawn a boss straight from a BNpcBase id (for trials with no battle-npc dropin). The
        // actor is registered as a director boss (anchors the encounter timeline on aggro) and its
        // `layout_id` is set to `base_id`, so `bosses()` returns it and `onActorDeath` can match it.
        methods.add_method_mut(
            "spawn_boss_base",
            |lua, this, (base_id, name_id, hp, level, position, rotation): (u32, u32, u32, u8, Value, f32)| {
                let position: Position = lua.from_value(position)?;
                this.tasks.push(LuaDirectorTask::SpawnBossByBase {
                    base_id,
                    name_id,
                    hp,
                    level,
                    position,
                    rotation,
                });
                Ok(())
            },
        );
        // Pre-spawn a pool of invisible helper actors used as omen / AoE cast sources, so the boss
        // isn't locked casting and charge actions don't drag it. Call once at battle start; the
        // helpers are ready by the time the first mechanic fires. `aoe_*({omen = true})` then routes
        // through them round-robin.
        methods.add_method_mut(
            "spawn_omen_pool",
            |lua, this, (count, base_id, name_id, position): (u32, u32, u32, Option<Value>)| {
                let position = match position {
                    Some(v) => lua.from_value(v)?,
                    None => Position::default(),
                };
                for _ in 0..count {
                    this.tasks.push(LuaDirectorTask::SpawnOmenHelper {
                        base_id,
                        name_id,
                        position,
                    });
                }
                Ok(())
            },
        );
        // Show/hide an actor (e.g. a boss leaving the field for a mechanic, or a clone idling hidden).
        // Retail uses ActorControl 414 (ToggleVisibility), which controls the client's bit16 — the
        // model's actual visibility. `duration` is the fade length (0 = instant).
        methods.add_method_mut(
            "set_visible",
            |_, this, (actor, visible, duration): (u32, bool, Option<f32>)| {
                this.tasks.push(LuaDirectorTask::SendActorControl {
                    actor_id: ObjectId(actor),
                    category: ActorControlCategory::ToggleVisibility {
                        visible,
                        duration: duration.unwrap_or(0.0),
                    },
                });
                Ok(())
            },
        );
        // Play a one-shot ActionTimeline animation on an actor (no cast bar / damage). Used for
        // mechanic flourishes like a boss's jump-away and landing during Crimson Cyclone (retail
        // plays ActionTimeline 0x008C jumping up, 0x008D landing — see parse_boss_timeline PATE).
        methods.add_method_mut("play_timeline", |_, this, (actor, timeline_id): (u32, u32)| {
            this.tasks.push(LuaDirectorTask::SendActorControl {
                actor_id: ObjectId(actor),
                category: ActorControlCategory::PlayActionTimeline { timeline_id },
            });
            Ok(())
        });
        // Force the client to re-read an actor's transform (position/rotation) and state. Retail sends
        // this (ActorControl 415) right after a SetPos/move so the client actually applies the new
        // facing — without it a moved actor (especially a hidden one just shown) keeps its stale
        // on-screen rotation (the Crimson Cyclone clones charging the wrong way). Call right after
        // `move_actor`.
        methods.add_method_mut("force_state_refresh", |_, this, actor: u32| {
            this.tasks.push(LuaDirectorTask::SendActorControl {
                actor_id: ObjectId(actor),
                category: ActorControlCategory::ForceStateRefresh {},
            });
            Ok(())
        });
        // Toggle whether an actor can be targeted (pair with `set_visible` when a boss leaves).
        methods.add_method_mut("set_targetable", |_, this, (actor, targetable): (u32, bool)| {
            this.tasks.push(LuaDirectorTask::SendActorControl {
                actor_id: ObjectId(actor),
                category: ActorControlCategory::Targetable { targetable },
            });
            Ok(())
        });
        // Pause/resume an NPC's behavior (chase + auto-attack). Use `false` while a boss is off doing
        // a mechanic so it stops hitting players; `true` to resume.
        methods.add_method_mut("set_ai_enabled", |_, this, (actor, enabled): (u32, bool)| {
            this.tasks.push(LuaDirectorTask::SetAiPaused {
                actor_id: ObjectId(actor),
                paused: !enabled,
            });
            Ok(())
        });
        // Teleport an actor to a position/rotation (e.g. a clone to its charge start).
        methods.add_method_mut(
            "move_actor",
            |lua, this, (actor, position, rotation): (u32, Value, f32)| {
                let position: Position = lua.from_value(position)?;
                this.tasks.push(LuaDirectorTask::MoveActor {
                    actor_id: ObjectId(actor),
                    position,
                    rotation,
                });
                Ok(())
            },
        );
        // Pre-spawn a pool of visible clone actors (hidden until shown). Drive each by index with
        // `director:clone(i)`.
        methods.add_method_mut(
            "spawn_clone_pool",
            |lua,
             this,
             (count, base_id, name_id, position, rotation): (
                u32,
                u32,
                u32,
                Option<Value>,
                Option<f32>,
            )| {
                let position = match position {
                    Some(v) => lua.from_value(v)?,
                    None => Position::default(),
                };
                let rotation = rotation.unwrap_or(0.0);
                for _ in 0..count {
                    this.tasks.push(LuaDirectorTask::SpawnClone {
                        base_id,
                        name_id,
                        position,
                        rotation,
                    });
                }
                Ok(())
            },
        );
        // Returns the actor id of the `index`-th (1-based) clone in the pool, or nil.
        methods.add_method("clone", |_, this, index: usize| {
            Ok(this.clones.get(index.wrapping_sub(1)).map(|id| id.0))
        });
        methods.add_method_mut("spawn_treasure", |_, this, id: u32| {
            this.tasks.push(LuaDirectorTask::SpawnTreasure { id });
            Ok(())
        });
        methods.add_method_mut("variant_vote_route", |_, this, npc_route: u32| {
            this.tasks
                .push(LuaDirectorTask::VariantVoteRoute { npc_route });
            Ok(())
        });
        methods.add_method_mut("play_cutscene", |_, this, cutscene_id: u32| {
            this.tasks
                .push(LuaDirectorTask::PlayCutscene { cutscene_id });
            Ok(())
        });
        methods.add_method_mut("update_shortcut", |_, this, poprange_id: u32| {
            // Show the shortcut object because it's hidden by default
            this.tasks.push(LuaDirectorTask::ShowEObj {
                base_id: EOBJ_SHORTCUT,
            });
            this.tasks
                .push(LuaDirectorTask::UpdateShortcut { poprange_id });
            Ok(())
        });
        methods.add_method_mut("use_shortcut", |_, this, actor_id: u32| {
            this.tasks.push(LuaDirectorTask::UseShortcut {
                actor_id: ObjectId(actor_id),
            });
            Ok(())
        });
        methods.add_method_mut("complete_duty", |_, this, _: ()| {
            // Show the exit object
            this.tasks
                .push(LuaDirectorTask::ShowEObj { base_id: EOBJ_EXIT });
            this.tasks.push(LuaDirectorTask::CompleteDuty {});
            Ok(())
        });
        methods.add_method_mut("map_effect", |_, this, (index, timeline_id): (u32, u32)| {
            this.tasks
                .push(LuaDirectorTask::MapEffect { index, timeline_id });
            Ok(())
        });
        // Schedule a callback at an absolute battle-elapsed time (seconds since battle start).
        methods.add_method_mut("at", |lua, this, (t, f): (f64, Function)| {
            let callback = lua.create_registry_value(f)?;
            this.pending_schedule.push(PendingSchedule {
                at_secs: t,
                callback,
                repeat: None,
            });
            Ok(())
        });
        // Schedule a callback `dt` seconds from now (relative to current battle elapsed).
        methods.add_method_mut("schedule", |lua, this, (dt, f): (f64, Function)| {
            let callback = lua.create_registry_value(f)?;
            this.pending_schedule.push(PendingSchedule {
                at_secs: this.elapsed_secs + dt,
                callback,
                repeat: None,
            });
            Ok(())
        });
        // Schedule a callback every `interval` seconds, `count` times. The callback receives
        // `(director, index)` with a zero-based index.
        methods.add_method_mut(
            "every",
            |lua, this, (interval, count, f): (f64, u32, Function)| {
                if count == 0 {
                    return Ok(());
                }
                let callback = lua.create_registry_value(f)?;
                this.pending_schedule.push(PendingSchedule {
                    at_secs: this.elapsed_secs + interval,
                    callback,
                    repeat: Some(RepeatSpec {
                        interval,
                        remaining: count,
                        index: 0,
                    }),
                });
                Ok(())
            },
        );

        // --- Encounter-scoped global vars ---
        methods.add_method_mut("set_var", |_, this, (key, value): (String, Value)| {
            match EncounterVar::from_lua(&value) {
                Some(v) => {
                    this.vars.insert(key, v);
                }
                None => {
                    this.vars.remove(&key);
                }
            }
            Ok(())
        });
        methods.add_method("get_var", |lua, this, key: String| {
            match this.vars.get(&key) {
                Some(v) => v.to_lua(lua),
                None => Ok(Value::Nil),
            }
        });
        methods.add_method_mut("clear_var", |_, this, key: String| {
            this.vars.remove(&key);
            Ok(())
        });
        methods.add_method_mut("clear_vars", |_, this, ()| {
            this.vars.clear();
            Ok(())
        });

        // --- Encounter-scoped per-player vars (keyed by actor id) ---
        methods.add_method_mut(
            "set_player_var",
            |_, this, (player, key, value): (u32, String, Value)| {
                let entry = this.player_vars.entry(ObjectId(player)).or_default();
                match EncounterVar::from_lua(&value) {
                    Some(v) => {
                        entry.insert(key, v);
                    }
                    None => {
                        entry.remove(&key);
                    }
                }
                Ok(())
            },
        );
        methods.add_method(
            "get_player_var",
            |lua, this, (player, key): (u32, String)| match this
                .player_vars
                .get(&ObjectId(player))
                .and_then(|m| m.get(&key))
            {
                Some(v) => v.to_lua(lua),
                None => Ok(Value::Nil),
            },
        );
        methods.add_method_mut(
            "clear_player_var",
            |_, this, (player, key): (u32, String)| {
                if let Some(m) = this.player_vars.get_mut(&ObjectId(player)) {
                    m.remove(&key);
                }
                Ok(())
            },
        );
        methods.add_method_mut("clear_player_vars", |_, this, player: u32| {
            this.player_vars.remove(&ObjectId(player));
            Ok(())
        });
        // Returns the actor ids of all players whose `key` var equals `value`.
        methods.add_method(
            "players_with_var",
            |_, this, (key, value): (String, Value)| {
                let target = EncounterVar::from_lua(&value);
                let ids: Vec<u32> = this
                    .player_vars
                    .iter()
                    .filter(|(_, m)| m.get(&key) == target.as_ref())
                    .map(|(id, _)| id.0)
                    .collect();
                Ok(ids)
            },
        );

        // --- Actor queries (read from this tick's snapshot) ---
        // All living player actor ids.
        methods.add_method("players", |_, this, ()| {
            let ids: Vec<u32> = this
                .actors
                .iter()
                .filter(|a| a.is_player)
                .map(|a| a.id.0)
                .collect();
            Ok(ids)
        });
        // Living player actor ids (hp > 0).
        methods.add_method("alive_players", |_, this, ()| {
            let ids: Vec<u32> = this
                .actors
                .iter()
                .filter(|a| a.is_player && a.alive())
                .map(|a| a.id.0)
                .collect();
            Ok(ids)
        });
        // Registered boss actor ids that are still alive.
        methods.add_method("bosses", |_, this, ()| {
            let ids: Vec<u32> = this
                .actors
                .iter()
                .filter(|a| a.is_boss && a.alive())
                .map(|a| a.id.0)
                .collect();
            Ok(ids)
        });
        // World position of an actor as a Lua table {x,y,z}, or nil if unknown.
        methods.add_method("position", |lua, this, actor_id: u32| {
            match this.actor(actor_id) {
                Some(a) => Ok(Value::Table(position_table(lua, a.position)?)),
                None => Ok(Value::Nil),
            }
        });
        // Facing (yaw, radians) of an actor, or 0.
        methods.add_method("rotation", |_, this, actor_id: u32| {
            Ok(this.actor(actor_id).map(|a| a.rotation).unwrap_or(0.0))
        });
        // Current hp of an actor.
        methods.add_method("hp", |_, this, actor_id: u32| {
            Ok(this.actor(actor_id).map(|a| a.hp).unwrap_or(0))
        });
        methods.add_method("max_hp", |_, this, actor_id: u32| {
            Ok(this.actor(actor_id).map(|a| a.max_hp).unwrap_or(0))
        });
        // Hp percentage [0,100] of an actor.
        methods.add_method("hp_percent", |_, this, actor_id: u32| {
            Ok(this
                .actor(actor_id)
                .map(|a| {
                    if a.max_hp == 0 {
                        0.0
                    } else {
                        (a.hp as f64 / a.max_hp as f64) * 100.0
                    }
                })
                .unwrap_or(0.0))
        });
        // True if the actor exists in this tick's snapshot and has hp > 0.
        methods.add_method("is_alive", |_, this, actor_id: u32| {
            Ok(this.actor(actor_id).map(|a| a.alive()).unwrap_or(false))
        });
        // Horizontal (X-Z) distance between two actors, or a large value if either is missing.
        methods.add_method("distance", |_, this, (a, b): (u32, u32)| {
            let (Some(pa), Some(pb)) = (this.actor(a), this.actor(b)) else {
                return Ok(f32::MAX);
            };
            let dx = pa.position.0.x - pb.position.0.x;
            let dz = pa.position.0.z - pb.position.0.z;
            Ok((dx * dx + dz * dz).sqrt())
        });

        // --- Helper actors ---
        // Spawn one named helper actor from a base BNpc id at a position (table {x,y,z}) with an
        // optional rotation. Returns its actor id.
        methods.add_method_mut(
            "spawn_helper",
            |lua, this, (name, bnpc_id, position, rotation): (String, u32, Value, Option<f32>)| {
                let position: Position = lua.from_value(position)?;
                let actor_id = ObjectId(fastrand::u32(..));
                this.helpers.insert(name, actor_id);
                this.tasks.push(LuaDirectorTask::SpawnHelper {
                    actor_id,
                    bnpc_id,
                    position,
                    rotation: rotation.unwrap_or(0.0),
                });
                Ok(actor_id.0)
            },
        );
        // Spawn many helpers from a list of {name, bnpc, pos={x,y,z}, rotation?}. Returns ids.
        methods.add_method_mut("spawn_helpers", |lua, this, defs: Vec<Table>| {
            let mut ids = Vec::with_capacity(defs.len());
            for def in defs {
                let name: String = def.get("name")?;
                let bnpc_id: u32 = def.get("bnpc")?;
                let position: Position = lua.from_value(def.get("pos")?)?;
                let rotation: f32 = def.get("rotation").unwrap_or(0.0);
                let actor_id = ObjectId(fastrand::u32(..));
                this.helpers.insert(name, actor_id);
                this.tasks.push(LuaDirectorTask::SpawnHelper {
                    actor_id,
                    bnpc_id,
                    position,
                    rotation,
                });
                ids.push(actor_id.0);
            }
            Ok(ids)
        });
        // Register an existing actor id under a name (e.g. a boss or add).
        methods.add_method_mut("register", |_, this, (name, actor_id): (String, u32)| {
            this.helpers.insert(name, ObjectId(actor_id));
            Ok(())
        });
        // Look up a named actor's id, or nil.
        methods.add_method("actor_id", |_, this, name: String| {
            Ok(this.helpers.get(&name).map(|id| id.0))
        });
        // Despawn an actor by id.
        methods.add_method_mut("despawn", |_, this, actor_id: u32| {
            this.tasks.push(LuaDirectorTask::DespawnActor {
                actor_id: ObjectId(actor_id),
            });
            // Drop any name pointing at it.
            this.helpers.retain(|_, id| id.0 != actor_id);
            Ok(())
        });
        // Despawn a named helper.
        methods.add_method_mut("despawn_registered", |_, this, name: String| {
            if let Some(actor_id) = this.helpers.remove(&name) {
                this.tasks.push(LuaDirectorTask::DespawnActor { actor_id });
            }
            Ok(())
        });
        // Despawn every registered helper.
        methods.add_method_mut("despawn_all_helpers", |_, this, ()| {
            for (_, actor_id) in this.helpers.drain() {
                this.tasks.push(LuaDirectorTask::DespawnActor { actor_id });
            }
            Ok(())
        });

        // --- Cast & AoE ---
        // Show an enemy cast bar from `source` for `action`, lasting `cast_time` seconds, aimed at
        // `target` (defaults to the source). Pure presentation; pair with an `aoe_*` for damage.
        methods.add_method_mut(
            "cast",
            |lua,
             this,
             (source, action, cast_time, target, dest): (
                u32,
                u32,
                f32,
                Option<u32>,
                Option<Value>,
            )| {
                this.tasks.push(LuaDirectorTask::CastBar {
                    source_id: ObjectId(source),
                    action_id: action,
                    cast_time,
                    target_id: ObjectId(target.unwrap_or(source)),
                });
                // Send a completing effect when the cast bar finishes, so the caster returns to idle.
                // Retail sends a DOWN_Effect at cast end; without it the actor is stuck in the cast
                // pose (the "卡cast动画" symptom). Radius 0 / damage 0 → animation only, hits nobody.
                //
                // `dest` is the action's TargetPos. For a directional charge (Crimson Cyclone's 457)
                // the client dashes the caster *toward this position*, NOT along its facing — retail's
                // 457 always carries TargetPos = arena centre, so the clone charges through the middle.
                // Default (no dest) = the caster's own spot, correct for in-place casts (eruption tell).
                let origin = match dest {
                    Some(v) => lua.from_value(v)?,
                    None => this.actor(source).map(|a| a.position).unwrap_or_default(),
                };
                let rotation = this.actor(source).map(|a| a.rotation).unwrap_or(0.0);
                this.tasks.push(LuaDirectorTask::ResolveAoe(PendingAoe {
                    shape: AoeShape::Circle { radius: 0.0 },
                    origin,
                    rotation,
                    delay: cast_time,
                    action_id: action,
                    damage: 0,
                    source_id: ObjectId(source),
                    effect_source: None,
                }));
                Ok(())
            },
        );
        // Play an action's animation on an actor with NO cast bar and NO damage — the way retail shows
        // an instant cleave (e.g. Incinerate: a DOWN_Effect / CST!, never a CST+). The animation plays
        // from the actor; pair it with a delayed `aoe_*` (action = 0) that lands the damage a beat
        // later so the floating number isn't stuck behind the long action animation.
        methods.add_method_mut(
            "play_action",
            |lua, this, (actor, action, dest): (u32, u32, Option<Value>)| {
                let origin = match dest {
                    Some(v) => lua.from_value(v)?,
                    None => this.actor(actor).map(|a| a.position).unwrap_or_default(),
                };
                let rotation = this.actor(actor).map(|a| a.rotation).unwrap_or(0.0);
                this.tasks.push(LuaDirectorTask::ResolveAoe(PendingAoe {
                    shape: AoeShape::Circle { radius: 0.0 },
                    origin,
                    rotation,
                    delay: 0.0,
                    action_id: action,
                    damage: 0,
                    source_id: ObjectId(actor),
                    effect_source: None,
                }));
                Ok(())
            },
        );
        // Circle AoE centered on `pos` (table) or, if omitted, on the source. Params table:
        // {source, radius, delay, action?, damage?, pos?}.
        methods.add_method_mut("aoe_circle", |lua, this, params: Table| {
            let shape = AoeShape::Circle {
                radius: params.get("radius")?,
            };
            this.push_aoe(lua, params, shape)
        });
        // Donut AoE: {source, inner, outer, delay, action?, damage?, pos?}.
        methods.add_method_mut("aoe_donut", |lua, this, params: Table| {
            let shape = AoeShape::Donut {
                inner: params.get("inner")?,
                outer: params.get("outer")?,
            };
            this.push_aoe(lua, params, shape)
        });
        // Forward rectangle / line AoE: {source, length, width, delay, action?, damage?, pos?,
        // rotation?}.
        methods.add_method_mut("aoe_rect", |lua, this, params: Table| {
            let shape = AoeShape::Rect {
                length: params.get("length")?,
                width: params.get("width")?,
            };
            this.push_aoe(lua, params, shape)
        });
        // Forward cone AoE: {source, radius, angle, delay, action?, damage?, pos?, rotation?}.
        methods.add_method_mut("aoe_cone", |lua, this, params: Table| {
            let shape = AoeShape::Cone {
                radius: params.get("radius")?,
                angle: params.get("angle")?,
            };
            this.push_aoe(lua, params, shape)
        });
        // Raidwide: hits every alive player. {source, delay, action?, damage?}.
        methods.add_method_mut("raidwide", |lua, this, params: Table| {
            this.push_aoe(lua, params, AoeShape::Everyone)
        });
    }
}

/// Build a Lua `{x,y,z}` table from a position.
fn position_table(lua: &Lua, pos: Position) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("x", pos.0.x)?;
    table.set("y", pos.0.y)?;
    table.set("z", pos.0.z)?;
    Ok(table)
}

impl LuaDirector {
    /// Look up an actor snapshot by id.
    fn actor(&self, actor_id: u32) -> Option<&ActorSnapshot> {
        self.actors.iter().find(|a| a.id.0 == actor_id)
    }

    /// Shared body for the `aoe_*`/`raidwide` methods: read the common params off the table, capture
    /// the origin/rotation now, and queue a `ResolveAoe` task.
    fn push_aoe(&mut self, lua: &Lua, params: Table, shape: AoeShape) -> mlua::Result<()> {
        let source_id = ObjectId(params.get::<u32>("source")?);

        // Origin: explicit `pos` table, else the source actor's current position.
        let origin = match params.get::<Value>("pos")? {
            Value::Nil => self
                .actor(source_id.0)
                .map(|a| a.position)
                .unwrap_or_default(),
            value => lua.from_value(value)?,
        };

        // Rotation: explicit `rotation`, else the source actor's facing.
        let rotation = match params.get::<Option<f32>>("rotation")? {
            Some(r) => r,
            None => self.actor(source_id.0).map(|a| a.rotation).unwrap_or(0.0),
        };

        let delay: f32 = params.get("delay").unwrap_or(0.0);
        let action_id: u32 = params.get("action").unwrap_or(0);
        let damage: u32 = params.get("damage").unwrap_or(0);
        // Cast time for the omen, separate from the resolve `delay`. Defaults to `delay` (ground
        // telegraph shows for the whole windup). Set to 0 for an instant boss animation with the
        // damage still landing `delay` later (e.g. a no-cast cleave like Incinerate).
        let omen_cast: f32 = params.get("omen_cast").unwrap_or(delay);

        // Optional ground telegraph (omen). `omen = true` routes through an off-arena helper from
        // the pool (round-robin) so the boss isn't locked casting and charge actions don't drag it;
        // `omen = <id>` uses a specific actor. The chosen helper also becomes the AoE's
        // `effect_source` so the resolve animates from it, not the boss.
        let want_omen = matches!(params.get::<Value>("omen")?, Value::Boolean(true));
        let explicit: Option<ObjectId> = match params.get::<Value>("omen")? {
            Value::Integer(id) => Some(ObjectId(id as u32)),
            _ => None,
        };
        let caster = if let Some(id) = explicit {
            Some(id)
        } else if want_omen && !self.omen_helpers.is_empty() {
            let id = self.omen_helpers[self.omen_rr % self.omen_helpers.len()];
            self.omen_rr = self.omen_rr.wrapping_add(1);
            Some(id)
        } else if want_omen {
            Some(source_id) // no pool yet — fall back to the source (boss will be locked casting)
        } else {
            None
        };

        // Directional shapes (rect/cone) are cast self-targeted so the client draws them forward
        // from the caster; circles/donuts are location-placed (no-target + position).
        let self_targeted = matches!(shape, AoeShape::Rect { .. } | AoeShape::Cone { .. });
        if let Some(caster) = caster
            && action_id != 0
            && delay > 0.0
        {
            self.tasks.push(LuaDirectorTask::OmenCast {
                caster_id: caster,
                action_id,
                cast_time: omen_cast,
                position: origin,
                rotation,
                self_targeted,
            });
        }

        // Animate the resolve from the helper (if one was used and it isn't the boss itself).
        let effect_source = caster.filter(|c| *c != source_id);

        self.tasks.push(LuaDirectorTask::ResolveAoe(PendingAoe {
            shape,
            origin,
            rotation,
            delay,
            action_id,
            damage,
            source_id,
            effect_source,
        }));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DirectorBossState {
    /// The boss arena is open and nothing has happened yet.
    Open,
    /// The boss arena is closing soon, because the boss was aggravated.
    Aggro,
    /// The boss arena is closed.
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectorBoss {
    state: DirectorBossState,
    actor_id: ObjectId,
    wall_id: u32,
    line_id: u32,
    place_name: u32,
}

#[derive(Debug)]
pub struct DirectorData {
    pub id: HandlerId,
    pub flag: u8,
    pub data: [u8; 10],
    /// Lua state for this director.
    pub lua: KawariLua,
    pub tasks: Vec<LuaDirectorTask>,
    /// List of alive bosses and their data.
    pub bosses: HashMap<u32, DirectorBoss>,
    /// What the shortcut is currently pointing to.
    pub shortcut_poprange_id: Option<u32>,
    /// Wall-clock start of the encounter; `None` until the boss first enters combat. Reset on wipe.
    pub battle_started_at: Option<Instant>,
    /// Set by `start_battle`, consumed by the next `encounter_tick`: defers `onBattleStart` until
    /// after the actor snapshot has been built, so queries like `alive_players()` see the players
    /// (the boss aggros from `npc_behavior` before any snapshot exists, hence the deferral).
    pub pending_battle_start: bool,
    /// Seconds since `battle_started_at`, recomputed from `Instant` each encounter tick (never a
    /// running tick count, so it doesn't drift with tick jitter).
    pub elapsed_secs: f64,
    /// Time of the previous encounter tick, used to compute `dt`.
    pub last_tick_at: Option<Instant>,
    /// Pending timeline events scheduled via `director:at/schedule/every`, sorted ascending.
    pub scheduler: Vec<ScheduledEvent>,
    /// When the deaggro reset condition first became true, used to require it to hold for a short
    /// grace before resetting (so a 1-tick hate-list flicker from the NPC leash logic doesn't reset
    /// the timeline mid-fight). Cleared whenever the boss has hate again.
    pub deaggro_since: Option<Instant>,
    /// Encounter-scoped global vars (random branches, round counters, object lists, call-out pools).
    pub vars: HashMap<String, EncounterVar>,
    /// Encounter-scoped per-player vars (call-outs, groups, mechanic state), keyed by actor id.
    pub player_vars: HashMap<ObjectId, HashMap<String, EncounterVar>>,
    /// Named helper actor registry (name -> actor id), created via `director:spawn_helper(s)`.
    pub helpers: HashMap<String, ObjectId>,
    /// Pool of invisible helper actors that cast omens / AoE effects on the boss's behalf (created
    /// via `director:spawn_omen_pool`). Keeps the boss free instead of locking it in a cast.
    pub omen_helpers: Vec<ObjectId>,
    /// Round-robin cursor into `omen_helpers`.
    pub omen_rr: usize,
    /// Pool of visible clone actors (created via `director:spawn_clone_pool`), e.g. Crimson Cyclone
    /// chargers. The Lua script drives each one by index via `director:clone(i)`.
    pub clones: Vec<ObjectId>,
    /// Snapshot of the instance's actors refreshed at the start of each encounter tick, so Lua
    /// position/hp queries are consistent within a tick without re-borrowing the live instance.
    pub current_actors: Vec<ActorSnapshot>,
}

impl DirectorData {
    pub fn setup(&mut self) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onSetup")?;

                func.call::<()>(data)?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onSetup: {err:?}");
        }
    }

    pub fn gimmick_accessor(&mut self, actor_id: ObjectId, id: u32, params: &[i32]) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onGimmickAccessor")?;

                func.call::<()>((data, actor_id.0, id, params))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onGimmickAccessor: {err:?}");
        }
    }

    pub fn event_action_cast(&mut self, actor_id: ObjectId, target: ObjectId) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onEventActionCast")?;

                func.call::<()>((data, actor_id.0, target.0))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onEventActionCast: {err:?}");
        }
    }

    pub fn on_actor_death(&mut self, bnpc_id: u32, position: Position) {
        // Only unseal a boss arena if one was configured (dungeon bosses). Trial bosses spawned via
        // `spawn_boss_base` have no wall (wall_id == 0); unsealing it would HideEObj(0) and emit a
        // blank "<place> seal lifted" log message.
        if let Some(boss) = self.bosses.get(&bnpc_id)
            && boss.wall_id != 0
        {
            self.unseal_boss_wall(boss.wall_id, boss.line_id, boss.place_name);
        }

        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onActorDeath")?;

                func.call::<()>((data, bnpc_id, position))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onActorDeath: {err:?}");
        }
    }

    /// Actor ids of every boss registered via `spawn_boss`. Used so boss NPCs can be exempted from
    /// the distance-based leash — a boss in a sealed arena should stay engaged until a wipe.
    pub fn boss_actor_ids(&self) -> Vec<ObjectId> {
        self.bosses.values().map(|b| b.actor_id).collect()
    }

    pub fn build_var_segment(&self) -> ServerZoneIpcSegment {
        ServerZoneIpcSegment::new(ServerZoneIpcData::DirectorVars {
            handler_id: self.id,
            flag: self.flag,
            branch: 0,
            data: self.data,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
        })
    }

    fn create_lua_director(&self) -> LuaDirector {
        LuaDirector {
            data: self.data,
            tasks: Vec::new(),
            bosses: self.bosses.clone(),
            elapsed_secs: self.elapsed_secs,
            pending_schedule: Vec::new(),
            vars: self.vars.clone(),
            player_vars: self.player_vars.clone(),
            actors: self.current_actors.clone(),
            helpers: self.helpers.clone(),
            omen_helpers: self.omen_helpers.clone(),
            omen_rr: self.omen_rr,
            clones: self.clones.clone(),
        }
    }

    fn apply_lua_director(&mut self, lua: LuaDirector) {
        if self.data != lua.data {
            self.data = lua.data;
            self.tasks.push(LuaDirectorTask::SendVariables {});
        }
        if self.bosses != lua.bosses {
            self.bosses = lua.bosses;
        }
        self.tasks.extend_from_slice(&lua.tasks);
        for pending in lua.pending_schedule {
            self.scheduler.push(ScheduledEvent {
                at_secs: pending.at_secs,
                callback: pending.callback,
                repeat: pending.repeat,
            });
        }
        // Sync the working copies of the encounter vars back. Callbacks run sequentially under the
        // data lock, so a straight overwrite is safe.
        self.vars = lua.vars;
        self.player_vars = lua.player_vars;
        self.helpers = lua.helpers;
        self.omen_rr = lua.omen_rr;
    }

    /// Returns true if the Lua script defines a global function with this name.
    fn has_callback(&self, name: &str) -> bool {
        matches!(
            self.lua.0.globals().get::<Value>(name),
            Ok(Value::Function(_))
        )
    }

    /// Run an optional encounter lifecycle callback (`onBattleStart`/`onReset`) that takes only the
    /// director. Silently does nothing if the script doesn't define it.
    fn run_lifecycle_callback(&mut self, name: &str) {
        if !self.has_callback(name) {
            return;
        }
        let mut lua_director = self.create_lua_director();
        let err = self.lua.0.scope(|scope| {
            let data = scope.create_userdata_ref_mut(&mut lua_director)?;
            let func: Function = self.lua.0.globals().get(name)?;
            func.call::<()>(data)?;
            Ok(())
        });
        self.apply_lua_director(lua_director);
        if let Err(err) = err {
            tracing::warn!("Error during {name}: {err:?}");
        }
    }

    /// Begin the encounter timeline. Only the first call takes effect; later calls (e.g. a second
    /// boss aggroing) are ignored so `t = 0` stays anchored to the first pull.
    pub fn start_battle(&mut self) {
        if self.battle_started_at.is_some() {
            return;
        }
        let now = Instant::now();
        self.battle_started_at = Some(now);
        self.last_tick_at = Some(now);
        self.elapsed_secs = 0.0;
        // Don't fire onBattleStart here: this runs from `npc_behavior` (boss aggro) before any
        // encounter tick has built the actor snapshot, so queries like `alive_players()` would see
        // nothing. The next `encounter_tick` runs it once the snapshot is ready.
        self.pending_battle_start = true;
    }

    /// Clear all encounter timeline state (wipe/reset/duty complete). Frees scheduled callbacks from
    /// the Lua registry so they don't leak, fires `onReset`, and re-opens the boss arenas so the next
    /// pull re-triggers `start_battle`. NOTE: boss HP/position are not restored here yet — a full
    /// retail-style wipe recovery is a later refinement; the Lua `onReset`/`onBattleStart` can handle
    /// re-setup in the meantime.
    pub fn reset_encounter(&mut self) {
        self.battle_started_at = None;
        self.last_tick_at = None;
        self.elapsed_secs = 0.0;
        self.deaggro_since = None;
        self.pending_battle_start = false;
        self.vars.clear();
        self.player_vars.clear();
        // Despawn any helper actors the encounter spawned so they don't leak into the next pull.
        for (_, actor_id) in self.helpers.drain() {
            self.tasks.push(LuaDirectorTask::DespawnActor { actor_id });
        }
        for actor_id in self.omen_helpers.drain(..) {
            self.tasks.push(LuaDirectorTask::DespawnActor { actor_id });
        }
        for actor_id in self.clones.drain(..) {
            self.tasks.push(LuaDirectorTask::DespawnActor { actor_id });
        }
        self.omen_rr = 0;
        for event in self.scheduler.drain(..) {
            let _ = self.lua.0.remove_registry_value(event.callback);
        }
        for boss in self.bosses.values_mut() {
            boss.state = DirectorBossState::Open;
        }
        self.run_lifecycle_callback("onReset");
    }

    /// Stop the encounter timeline without wipe/reset semantics — used on duty completion so the
    /// scheduled `at`/`schedule`/`every` callbacks (e.g. a boss mechanic loop) stop firing once the
    /// boss is dead, without each script having to track a "finished" flag. Frees the Lua registry
    /// values. Does NOT fire `onReset` or reopen the arena (the duty is over, not being retried).
    pub fn stop_timeline(&mut self) {
        self.battle_started_at = None;
        self.last_tick_at = None;
        self.pending_battle_start = false;
        for event in self.scheduler.drain(..) {
            let _ = self.lua.0.remove_registry_value(event.callback);
        }
        for actor_id in self.omen_helpers.drain(..) {
            self.tasks.push(LuaDirectorTask::DespawnActor { actor_id });
        }
        for actor_id in self.clones.drain(..) {
            self.tasks.push(LuaDirectorTask::DespawnActor { actor_id });
        }
        self.omen_rr = 0;
    }

    /// Drive one encounter tick: fire any due scheduled events, then call `onTick(director, dt, t)`.
    /// Does nothing until the battle has started.
    pub fn encounter_advance(&mut self, now: Instant) {
        let Some(started) = self.battle_started_at else {
            return;
        };
        let dt = self
            .last_tick_at
            .map(|prev| now.saturating_duration_since(prev).as_secs_f64())
            .unwrap_or(0.0);
        self.last_tick_at = Some(now);
        self.elapsed_secs = now.saturating_duration_since(started).as_secs_f64();
        let t = self.elapsed_secs;

        self.fire_due_events(t);
        self.on_tick(dt, t);
    }

    /// Fire every scheduled event whose time has arrived, in ascending time order. Re-checks after
    /// each callback so events queued by a callback this tick still fire if they're already due.
    fn fire_due_events(&mut self, t: f64) {
        // Backstop against a script that schedules itself at a non-advancing time and would
        // otherwise spin forever within a single tick.
        let mut guard = 0;
        loop {
            self.scheduler.sort_by(|a, b| {
                a.at_secs
                    .partial_cmp(&b.at_secs)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let Some(pos) = self.scheduler.iter().position(|e| e.at_secs <= t) else {
                break;
            };
            let event = self.scheduler.remove(pos);
            let index = event.repeat.map(|r| r.index).unwrap_or(0);
            self.call_scheduled(&event.callback, index);

            match event.repeat {
                Some(mut repeat) if repeat.remaining > 1 => {
                    repeat.remaining -= 1;
                    repeat.index += 1;
                    self.scheduler.push(ScheduledEvent {
                        at_secs: event.at_secs + repeat.interval,
                        callback: event.callback,
                        repeat: Some(repeat),
                    });
                }
                _ => {
                    let _ = self.lua.0.remove_registry_value(event.callback);
                }
            }

            guard += 1;
            if guard > 4096 {
                tracing::warn!("fire_due_events exceeded 4096 events in one tick, bailing out");
                break;
            }
        }
    }

    /// Invoke a scheduled callback stored in the Lua registry, passing `(director, index)`.
    fn call_scheduled(&mut self, callback: &RegistryKey, index: u32) {
        let mut lua_director = self.create_lua_director();
        let err = self.lua.0.scope(|scope| {
            let data = scope.create_userdata_ref_mut(&mut lua_director)?;
            let func: Function = self.lua.0.registry_value(callback)?;
            func.call::<()>((data, index))?;
            Ok(())
        });
        self.apply_lua_director(lua_director);
        if let Err(err) = err {
            tracing::warn!("Error during scheduled event: {err:?}");
        }
    }

    /// Call the optional `onTick(director, dt, t)` callback.
    fn on_tick(&mut self, dt: f64, t: f64) {
        if !self.has_callback("onTick") {
            return;
        }
        let mut lua_director = self.create_lua_director();
        let err = self.lua.0.scope(|scope| {
            let data = scope.create_userdata_ref_mut(&mut lua_director)?;
            let func: Function = self.lua.0.globals().get("onTick")?;
            func.call::<()>((data, dt, t))?;
            Ok(())
        });
        self.apply_lua_director(lua_director);
        if let Err(err) = err {
            tracing::warn!("Error during onTick: {err:?}");
        }
    }

    /// Actually insert tasks to seal the boss wall.
    pub fn seal_boss_wall(&mut self, id: u32, place_name: u32) {
        if let Some(boss) = self.bosses.iter_mut().find(|x| x.1.wall_id == id) {
            self.tasks.push(LuaDirectorTask::LogMessage {
                id: 2013,
                params: vec![place_name],
            });
            self.tasks.push(LuaDirectorTask::ShowEObj { base_id: id });
            boss.1.state = DirectorBossState::Closed;
        }
    }

    /// Actually insert tasks to unseal the boss wall.
    pub fn unseal_boss_wall(&mut self, wall_id: u32, line_id: u32, place_name: u32) {
        self.tasks
            .push(LuaDirectorTask::HideEObj { base_id: wall_id });
        self.tasks
            .push(LuaDirectorTask::HideEObj { base_id: line_id });
        self.tasks.push(LuaDirectorTask::LogMessage {
            id: 2014,
            params: vec![place_name],
        });
    }

    pub fn on_actor_aggro(&mut self, id: u32) {
        let mut aggroed = false;
        if let Some(boss) = self.bosses.get_mut(&id)
            && boss.state == DirectorBossState::Open
        {
            // Only seal/announce a boss arena if one was configured (dungeon bosses). Trial bosses
            // spawned via `spawn_boss_base` have no wall (wall_id == 0), so skip the seal.
            if boss.wall_id != 0 {
                // TODO: is there times that are longer than 15 secs?
                self.tasks.push(LuaDirectorTask::LogMessage {
                    id: 2012,
                    params: vec![boss.place_name, 15],
                });
                self.tasks.push(LuaDirectorTask::SealBossWall {
                    actor_id: boss.actor_id,
                    id: boss.wall_id,
                    place_name: boss.place_name,
                    time_until: 15,
                });
            }
            boss.state = DirectorBossState::Aggro;
            aggroed = true;
        }

        // The boss first entering combat anchors the encounter timeline at t = 0. Done after the
        // borrow above ends; start_battle no-ops on subsequent boss aggros.
        if aggroed {
            self.start_battle();
        }
    }

    pub fn on_gimmick_rect(&mut self, id: u32) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onGimmickRect")?;

                func.call::<()>((data, id))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onGimmickRect: {err:?}");
        }
    }

    pub fn get_debug_shortcut(&self, id: u32) -> u32 {
        let run_script = || {
            self.lua.0.scope(|_| {
                let func: Function = self.lua.0.globals().get("getDebugShortcut")?;

                func.call::<u32>(id)
            })
        };
        match run_script() {
            Ok(pop_range_id) => pop_range_id,
            Err(err) => {
                tracing::warn!("Syntax error during getDebugShortcut: {err:?}");

                0
            }
        }
    }

    pub fn variant_vote(&mut self, vote: u32) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onVariantVote")?;

                func.call::<()>((data, vote))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onVariantVote: {err:?}");
        }
    }
}

/// Advance the encounter timeline for one high-frequency (~125ms) tick: fire due scheduled events
/// and call `onTick`. Also performs a minimal wipe/reset check. Queued director task draining is
/// done separately by `director_tick`.
pub fn encounter_tick(instance: &mut Instance, now: Instant) {
    // Cheap early-out unless an encounter is actually running.
    let running = matches!(&instance.director, Some(d) if d.battle_started_at.is_some());
    if !running {
        return;
    }

    // Decide whether the encounter should reset. Three signals, all read from instance state before
    // we take the mutable director borrow:
    //  - no players left in the instance (everyone left),
    //  - wipe: there are players but every one of them is dead,
    //  - deaggro: there's at least one living boss and all living bosses have dropped their hate
    //    (the boss leashed home), so the pull is over.
    let mut player_count = 0usize;
    let mut alive_players = 0usize;
    for actor in instance.actors.values() {
        if matches!(actor, NetworkedActor::Player { .. }) {
            player_count += 1;
            if actor.get_common_spawn().health_points > 0 {
                alive_players += 1;
            }
        }
    }
    let no_players = player_count == 0;
    let wipe = player_count > 0 && alive_players == 0;

    let boss_ids: Vec<ObjectId> = instance
        .director
        .as_ref()
        .map(|d| d.bosses.values().map(|b| b.actor_id).collect())
        .unwrap_or_default();
    let mut living_bosses = 0usize;
    let mut deaggroed_bosses = 0usize;
    for id in &boss_ids {
        if let Some(NetworkedActor::Npc {
            state,
            hate_list,
            spawn,
            ..
        }) = instance.find_actor(*id)
            && *state != NpcState::Dead
            && spawn.common.health_points > 0
        {
            living_bosses += 1;
            if hate_list.is_empty() {
                deaggroed_bosses += 1;
            }
        }
    }
    let deaggro = living_bosses > 0 && deaggroed_bosses == living_bosses;

    // Build this tick's actor snapshot (players + npcs) before the mutable director borrow, so Lua
    // position/hp queries during `onTick`/scheduled callbacks read a consistent picture.
    let boss_set: std::collections::HashSet<ObjectId> = boss_ids.iter().copied().collect();
    let snapshot: Vec<ActorSnapshot> = instance
        .actors
        .iter()
        .filter_map(|(id, actor)| match actor {
            NetworkedActor::Player { spawn, .. } => Some(ActorSnapshot {
                id: *id,
                is_player: true,
                is_boss: false,
                position: spawn.common.position,
                rotation: spawn.common.rotation,
                hp: spawn.common.health_points,
                max_hp: spawn.common.max_health_points,
            }),
            NetworkedActor::Npc { spawn, .. } => Some(ActorSnapshot {
                id: *id,
                is_player: false,
                is_boss: boss_set.contains(id),
                position: spawn.common.position,
                rotation: spawn.common.rotation,
                hp: spawn.common.health_points,
                max_hp: spawn.common.max_health_points,
            }),
            _ => None,
        })
        .collect();

    let Some(director) = &mut instance.director else {
        return;
    };
    director.current_actors = snapshot;

    // No players / full wipe are unambiguous — reset immediately. Deaggro keys off the boss hate
    // list, which the NPC leash logic can briefly clear and refill (a flicker), so require it to
    // hold for a short grace before resetting, to avoid wiping the timeline mid-fight.
    if no_players || wipe {
        director.reset_encounter();
        return;
    }
    if deaggro {
        let since = *director.deaggro_since.get_or_insert(now);
        if now.saturating_duration_since(since).as_secs_f64() >= 2.0 {
            director.reset_encounter();
            return;
        }
    } else {
        director.deaggro_since = None;
    }

    // Fire the deferred onBattleStart now that the snapshot (`current_actors`) is populated, so the
    // script's queries (`alive_players()` etc.) see the players. Runs before the first onTick.
    if director.pending_battle_start {
        director.pending_battle_start = false;
        director.run_lifecycle_callback("onBattleStart");
    }

    director.encounter_advance(now);
}

/// Perform any queued director tasks. Returns the AoEs that need to be resolved on a precise timer
/// (the caller owns the `data`/`network` handles needed to schedule and apply them).
pub fn director_tick(
    network: Arc<Mutex<NetworkState>>,
    gamedata: Arc<Mutex<GameData>>,
    instance: &mut Instance,
) -> Vec<PendingAoe> {
    let tasks = if let Some(director) = &instance.director {
        director.tasks.clone()
    } else {
        return Vec::new();
    };

    let mut bosses = if let Some(director) = &instance.director {
        director.bosses.clone()
    } else {
        return Vec::new();
    };

    let mut pending_aoes = Vec::new();
    let director_id = instance.director.as_ref().unwrap().id;

    for task in &tasks {
        match task {
            LuaDirectorTask::HideEObj { base_id } => {
                let Some(actor_id) = instance.find_object_by_eobj_id(*base_id) else {
                    tracing::warn!("Failed to find eobj {base_id} for HideEObj, it won't despawn!");
                    continue;
                };

                let state = EventState::UNK1 | EventState::UNK2 | EventState::UNK3;

                let mut network = network.lock();
                network.send_ac_in_range_instance(
                    instance,
                    actor_id,
                    ActorControlCategory::SetEventState { state },
                );

                // Update invisibility flags for next spawn
                if let Some(NetworkedActor::Object { object, .. }) =
                    instance.find_actor_mut(actor_id)
                {
                    object.event_state = state;
                }
            }
            LuaDirectorTask::ShowEObj { base_id } => {
                let Some(actor_id) = instance.find_object_by_eobj_id(*base_id) else {
                    tracing::warn!("Failed to find eobj {base_id} for ShowEObj, it won't despawn!");
                    continue;
                };

                let state = EventState::empty();

                let mut network = network.lock();
                network.send_ac_in_range_instance(
                    instance,
                    actor_id,
                    ActorControlCategory::SetEventState { state },
                );

                // Update invisibility flags for next spawn
                if let Some(NetworkedActor::Object { object, .. }) =
                    instance.find_actor_mut(actor_id)
                {
                    object.event_state = state;
                }
            }
            LuaDirectorTask::DeleteEObj { base_id } => {
                let Some(actor_id) = instance.find_object_by_eobj_id(*base_id) else {
                    tracing::warn!(
                        "Failed to find eobj {base_id} for DeleteEObj, it won't despawn!"
                    );
                    continue;
                };

                let mut network = network.lock();
                network.remove_actor(instance, actor_id);
            }
            LuaDirectorTask::SpawnEObj { base_id, position } => {
                if let Some(mut object) = instance.zone.get_event_object(*base_id) {
                    if let Some(position) = position {
                        object.position = *position;
                    }
                    instance.insert_object(object.entity_id, object, String::default()); // TODO: insert layer name
                } else {
                    tracing::warn!("Failed to find eobj {base_id} for SpawnEObj, it won't spawn!");
                }
            }
            LuaDirectorTask::SendVariables => {
                let vars = if let Some(director) = &instance.director {
                    director.build_var_segment()
                } else {
                    panic!("There's no way this could've happened!");
                };

                let mut network = network.lock();
                for id in instance.actors.keys() {
                    let Some((handle, _)) = network.get_by_actor_mut(*id) else {
                        continue;
                    };

                    let msg = FromServer::PacketSegment(vars.clone(), *id);
                    let _ = handle.send(msg.clone()); // TODO: use result
                }
            }
            LuaDirectorTask::AbandonDuty { actor_id } => {
                let mut network = network.lock();
                network.send_to_by_actor_id(
                    *actor_id,
                    FromServer::LeaveContent(),
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::BeginEventAction {
                actor_id,
                target,
                action_id,
            } => {
                let act = ActorControlCategory::EventAction {
                    unk1: 1,
                    id: *action_id,
                };

                let mut network = network.lock();
                network.send_to_by_actor_id(
                    *actor_id,
                    FromServer::ActorControlTarget(
                        *actor_id,
                        ObjectTypeId {
                            object_id: *target,
                            object_type: ObjectTypeKind::None,
                        },
                        act,
                    ),
                    DestinationNetwork::ZoneClients,
                );

                // TODO: set OccupiedInEvent?

                // TODO: don't hardcode this duration, take it from the EventAction sheet!
                instance.insert_task(
                    ClientId::default(),
                    *actor_id,
                    Duration::from_secs(2),
                    QueuedTaskData::CastEventAction { target: *target },
                );
            }
            LuaDirectorTask::FinishGimmickEvent { actor_id } => {
                let mut network = network.lock();
                network.send_to_by_actor_id(
                    *actor_id,
                    FromServer::FinishEvent(),
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::LogMessage { id, params } => {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LogMessage {
                    handler_id: director_id,
                    message_type: *id,
                    params_count: params.len() as u32,
                    item_id: params.first().copied().unwrap_or_default(),
                    item_quantity: params.get(1).copied().unwrap_or_default(),
                });

                let mut network = network.lock();
                network.send_to_instance(
                    ObjectId::default(),
                    instance,
                    FromServer::PacketSegment(ipc, ObjectId::default()), // TODO: how do we just send it from the player?
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::SpawnBattleNpc { id } => {
                if let Some(mut npc) = instance.zone.get_battle_npc(*id) {
                    npc.common.handler_id = director_id;
                    instance.insert_npc(ObjectId(fastrand::u32(..)), npc);
                } else {
                    tracing::warn!("Failed to find bnpc {id} for SpawnBattleNpc, it won't spawn!");
                }
            }
            LuaDirectorTask::GainEffect {
                actor_id,
                id,
                param,
                duration,
            } => {
                gain_effect_instance(
                    network.clone(),
                    ClientId::default(),
                    instance,
                    *actor_id,
                    *id,
                    *param,
                    *duration,
                    ObjectId::default(),
                    false, // Don't need to inform players here
                );
            }
            LuaDirectorTask::SetBGM { id } => {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(
                    ActorControlSelf {
                        category: ActorControlCategory::DirectorEvent {
                            handler_id: director_id,
                            event: DirectorEvent::SetBGM { bgm: *id },
                        },
                    },
                ));

                let mut network = network.lock();
                network.send_to_instance(
                    ObjectId::default(),
                    instance,
                    FromServer::PacketSegment(ipc, ObjectId::default()), // TODO: how do we just send it from the player?
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::SealBossWall {
                actor_id,
                id,
                place_name,
                time_until,
            } => {
                instance.insert_task(
                    ClientId::default(),
                    *actor_id,
                    Duration::from_secs(*time_until as u64),
                    QueuedTaskData::SealBossWall {
                        id: *id,
                        place_name: *place_name,
                    },
                );
            }
            LuaDirectorTask::SpawnBoss {
                bnpc_id,
                wall_id,
                line_id,
                place_name,
            } => {
                if let Some(mut npc) = instance.zone.get_battle_npc(*bnpc_id) {
                    npc.common.handler_id = director_id;

                    let actor_id = ObjectId(fastrand::u32(..));
                    instance.insert_npc(actor_id, npc);
                    bosses.insert(
                        *bnpc_id,
                        DirectorBoss {
                            state: DirectorBossState::Open,
                            actor_id,
                            wall_id: *wall_id,
                            line_id: *line_id,
                            place_name: *place_name,
                        },
                    );
                } else {
                    tracing::warn!("Failed to find bnpc {bnpc_id} for SpawnBoss, it won't spawn!");
                }
            }
            LuaDirectorTask::SpawnBossByBase {
                base_id,
                name_id,
                hp,
                level,
                position,
                rotation,
            } => {
                let bnpc = {
                    let mut game_data = gamedata.lock();
                    game_data.find_bnpc(*base_id).map(
                        |(model_chara, battalion, customize, rank, equip)| {
                            let equip_spawn =
                                game_data.get_npc_equip(equip as u32).unwrap_or_default();
                            (model_chara, battalion, customize, rank, equip_spawn)
                        },
                    )
                };
                if let Some((model_chara, battalion, customize, rank, equip_spawn)) = bnpc {
                    let actor_id = ObjectId(fastrand::u32(..));
                    let npc = SpawnNpc {
                        character_data_flags: CharacterDataFlag::HOSTILE,
                        character_data_icon: rank,
                        common: CommonSpawn {
                            handler_id: director_id,
                            base_id: *base_id,
                            name_id: *name_id,
                            // layout_id == base_id so `onActorDeath` / `bosses()` can match it.
                            layout_id: *base_id,
                            health_points: *hp,
                            max_health_points: *hp,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            battalion,
                            level: *level,
                            model_chara,
                            position: *position,
                            rotation: *rotation,
                            look: customize,
                            ..equip_spawn
                        },
                        ..Default::default()
                    };
                    instance.insert_npc(actor_id, npc);
                    // Trial bosses stand at a fixed spot until engaged — not Wander (which would let
                    // them drift / proximity-aggro). They still aggro normally when attacked (the
                    // hate-list transition isn't gated on Wander).
                    if let Some(NetworkedActor::Npc { state, .. }) =
                        instance.find_actor_mut(actor_id)
                    {
                        *state = NpcState::Stay;
                    }
                    bosses.insert(
                        *base_id,
                        DirectorBoss {
                            state: DirectorBossState::Open,
                            actor_id,
                            wall_id: 0,
                            line_id: 0,
                            place_name: 0,
                        },
                    );
                } else {
                    tracing::warn!(
                        "Failed to find BNpcBase {base_id} for SpawnBossByBase, it won't spawn!"
                    );
                }
            }
            LuaDirectorTask::SpawnTreasure { id } => {
                if let Some(mut treasure) = instance.zone.get_treasure(*id as u8) {
                    treasure.handler_id = director_id;

                    let actor_id = ObjectId(fastrand::u32(..));
                    instance.insert_treasure(actor_id, treasure);
                } else {
                    tracing::warn!(
                        "Failed to find treasure {id} for SpawnTreasure, it won't spawn!"
                    );
                }
            }
            LuaDirectorTask::VariantVoteRoute { npc_route } => {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(
                    ActorControlSelf {
                        category: ActorControlCategory::DirectorEvent {
                            handler_id: director_id,
                            event: DirectorEvent::VariantVoteRoute {
                                votes_needed: 1, // TODO: set to the number of players in the instance
                                npc_route: *npc_route,
                            },
                        },
                    },
                ));

                let mut network = network.lock();
                network.send_to_instance(
                    ObjectId::default(),
                    instance,
                    FromServer::PacketSegment(ipc, ObjectId::default()), // TODO: how do we just send it from the player?
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::PlayCutscene { cutscene_id } => {
                let mut network = network.lock();
                network.send_to_instance(
                    ObjectId::default(),
                    instance,
                    FromServer::PlayDirectorCutscene(*cutscene_id),
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::UpdateShortcut { poprange_id } => {
                instance.director.as_mut().unwrap().shortcut_poprange_id = Some(*poprange_id);
            }
            LuaDirectorTask::UseShortcut { actor_id } => {
                instance.insert_task(
                    ClientId::default(),
                    *actor_id,
                    Duration::from_secs(0),
                    QueuedTaskData::WarpToPopRange {
                        id: instance
                            .director
                            .as_ref()
                            .unwrap()
                            .shortcut_poprange_id
                            .unwrap(),
                    },
                );
            }
            LuaDirectorTask::CompleteDuty {} => {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(
                    ActorControlSelf {
                        category: ActorControlCategory::DirectorEvent {
                            handler_id: director_id,
                            event: DirectorEvent::DutyCompleted {
                                arg1: 0,
                                arg2: 0,
                                arg3: 0,
                                arg4: 0,
                            },
                        },
                    },
                ));

                {
                    let mut network = network.lock();
                    network.send_to_instance(
                        ObjectId::default(),
                        instance,
                        FromServer::PacketSegment(ipc, ObjectId::default()), // TODO: how do we just send it from the player?
                        DestinationNetwork::ZoneClients,
                    );
                }

                // Stop the mechanic timeline now the duty is over, so scheduled callbacks (e.g. a
                // boss cycle loop) don't keep firing. Scripts don't need their own "finished" flag.
                if let Some(director) = &mut instance.director {
                    director.stop_timeline();
                }

                // TODO: mark duty as completed for each player
            }
            LuaDirectorTask::MapEffect { index, timeline_id } => {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::DirectorMapEffect {
                    handler_id: director_id,
                    state: 1,
                    timeline_id: *timeline_id as u16,
                    index: *index as u8,
                });

                let mut network = network.lock();
                network.send_to_instance(
                    ObjectId::default(),
                    instance,
                    FromServer::PacketSegment(ipc, ObjectId::default()), // TODO: how do we just send it from the player?
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::SpawnHelper {
                actor_id,
                bnpc_id,
                position,
                rotation,
            } => {
                if let Some(mut npc) = instance.zone.get_battle_npc(*bnpc_id) {
                    npc.common.handler_id = director_id;
                    npc.common.position = *position;
                    npc.common.rotation = *rotation;
                    // The 300ms in-range visibility pass spawns the inserted NPC to clients.
                    instance.insert_npc(*actor_id, npc);
                } else {
                    tracing::warn!(
                        "Failed to find bnpc {bnpc_id} for SpawnHelper, it won't spawn!"
                    );
                }
            }
            LuaDirectorTask::DespawnActor { actor_id } => {
                instance.cancel_actor_tasks(*actor_id);
                let mut network = network.lock();
                network.remove_actor(instance, *actor_id);
            }
            LuaDirectorTask::CastBar {
                source_id,
                action_id,
                cast_time,
                target_id,
            } => {
                let position = instance
                    .find_actor(*source_id)
                    .map(|a| a.position())
                    .unwrap_or_default();
                let rotation = instance
                    .find_actor(*source_id)
                    .map(|a| a.rotation())
                    .unwrap_or(0.0);

                // Freeze the caster for the duration of the cast (cleared when the cast's effect
                // resolves) so it doesn't keep chasing/meleeing the player while "reading" the cast.
                if let Some(NetworkedActor::Npc { cast_locked, .. }) =
                    instance.find_actor_mut(*source_id)
                {
                    *cast_locked = true;
                }

                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorCast {
                    spell_id: *action_id as u16,
                    action_type: ActionType::Action,
                    omen_delay: 0,
                    action_id: *action_id,
                    cast_time: *cast_time,
                    target: *target_id,
                    rotation,
                    interruptible: false,
                    ballista_entity_id: ObjectId::default(),
                    position,
                });

                let mut network = network.lock();
                network.send_in_range_inclusive_instance(
                    *source_id,
                    instance,
                    FromServer::PacketSegment(ipc, *source_id),
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::ResolveAoe(pending) => {
                pending_aoes.push(pending.clone());
            }
            LuaDirectorTask::OmenCast {
                caster_id,
                action_id,
                cast_time,
                position,
                rotation,
                self_targeted,
            } => {
                // Move the (parked, off-arena) helper onto the AoE spot first. The resolution effect
                // animates from this helper, so without the move it would render where the helper was
                // parked (e.g. arena centre). Retail sends an ActorMove to the helper right after its
                // cast for exactly this. The helper is hidden (UNK flags) so the move isn't seen.
                let found = instance.find_actor_mut(*caster_id).is_some();
                if let Some(NetworkedActor::Npc { spawn, .. }) = instance.find_actor_mut(*caster_id) {
                    spawn.common.position = *position;
                    spawn.common.rotation = *rotation;
                }
                tracing::info!(
                    "[omen_cast] caster={:?} found={} action={} pos=({:.1},{:.1}) cast={:.1}",
                    caster_id, found, action_id, position.0.x, position.0.z, cast_time
                );

                let mut network = network.lock();
                network.send_in_range_inclusive_instance(
                    *caster_id,
                    instance,
                    FromServer::ActorMove(
                        *caster_id,
                        *position,
                        *rotation,
                        MoveAnimationType::empty(),
                        MoveAnimationState::empty(),
                        JumpState::empty(),
                    ),
                    DestinationNetwork::ZoneClients,
                );

                // The ground telegraph (omen) is just an ActorCast of the AoE action from this helper,
                // with `target = no-target` and `position` = where the AoE lands. The client draws the
                // omen shape from the action's data at that position (no separate omen/VFX packet).
                // Self-targeted (rect/cone): the omen extends forward from the caster. Location
                // (circle): no-target, omen drawn at `position`.
                let target = if *self_targeted {
                    *caster_id
                } else {
                    ObjectId::default()
                };
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorCast {
                    spell_id: *action_id as u16,
                    action_type: ActionType::Action,
                    omen_delay: 0,
                    action_id: *action_id,
                    cast_time: *cast_time,
                    target,
                    rotation: *rotation,
                    interruptible: false,
                    ballista_entity_id: ObjectId::default(),
                    position: *position,
                });

                network.send_in_range_inclusive_instance(
                    *caster_id,
                    instance,
                    FromServer::PacketSegment(ipc, *caster_id),
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::SpawnOmenHelper {
                base_id,
                name_id,
                position,
            } => {
                // The retail Ifrit helper is BNpcBase 434, but its nameplate name is the boss's
                // (BNpcName 1185 = 伊弗利特), so base_id and name_id are passed in separately. Spawned
                // invisible + non-hostile + Stay so it never aggros or shows; it only exists to be an
                // off-arena cast source.
                let bnpc = {
                    let mut game_data = gamedata.lock();
                    game_data.find_bnpc(*base_id).map(
                        |(model_chara, battalion, customize, _rank, equip)| {
                            let equip_spawn =
                                game_data.get_npc_equip(equip as u32).unwrap_or_default();
                            (model_chara, battalion, customize, equip_spawn)
                        },
                    )
                };
                if let Some((model_chara, battalion, customize, equip_spawn)) = bnpc {
                    let actor_id = ObjectId(fastrand::u32(..));
                    let npc = SpawnNpc {
                        // Retail's Ifrit helper (decoded from a native NpcSpawn) uses these exact
                        // display flags — UNK2 | UNK1 — to hide the actor + nameplate while STILL
                        // letting the client draw the omen from its cast. (INVISIBLE would suppress
                        // the omen; an empty model alone still shows a nameplate.) BNpcBase 434's
                        // ModelChara (480) is empty anyway. No HOSTILE flag → Stay sensing skips it.
                        common: CommonSpawn {
                            handler_id: director_id,
                            base_id: *base_id,
                            name_id: *name_id,
                            health_points: 1,
                            max_health_points: 1,
                            display_flags: DisplayFlag::UNK2 | DisplayFlag::UNK1,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            battalion,
                            model_chara,
                            position: *position,
                            look: customize,
                            ..equip_spawn
                        },
                        ..Default::default()
                    };
                    instance.insert_npc(actor_id, npc);
                    if let Some(NetworkedActor::Npc {
                        state, targetable, ..
                    }) = instance.find_actor_mut(actor_id)
                    {
                        *state = NpcState::Stay;
                        // Pure cast source: never a valid target, so player AoE doesn't hit it
                        // (it has 1 HP — an AoE would otherwise "kill" it mid-fight).
                        *targetable = false;
                    }
                    if let Some(director) = &mut instance.director {
                        director.omen_helpers.push(actor_id);
                    }
                } else {
                    tracing::warn!(
                        "Failed to find omen helper BNpcBase {base_id}; omens fall back to the boss"
                    );
                }
            }
            LuaDirectorTask::SendActorControl { actor_id, category } => {
                // Keep the stored `targetable`/`visible` in sync so they survive client re-spawns
                // (walked-in re-applies them).
                if let ActorControlCategory::Targetable { targetable } = category
                    && let Some(NetworkedActor::Npc { targetable: t, .. }) =
                        instance.find_actor_mut(*actor_id)
                {
                    *t = *targetable;
                }
                if let ActorControlCategory::ToggleVisibility { visible, .. } = category
                    && let Some(NetworkedActor::Npc { visible: v, .. }) =
                        instance.find_actor_mut(*actor_id)
                {
                    *v = *visible;
                }
                let mut network = network.lock();
                network.send_ac_in_range_instance(instance, *actor_id, category.clone());
            }
            LuaDirectorTask::SetAiPaused { actor_id, paused } => {
                if let Some(NetworkedActor::Npc {
                    ai_paused,
                    navmesh_target,
                    navmesh_path,
                    ..
                }) = instance.find_actor_mut(*actor_id)
                {
                    *ai_paused = *paused;
                    if *paused {
                        // Stop it mid-chase; it keeps its hate list and re-acquires on resume.
                        *navmesh_target = None;
                        navmesh_path.clear();
                    }
                }
            }
            LuaDirectorTask::MoveActor {
                actor_id,
                position,
                rotation,
            } => {
                if let Some(NetworkedActor::Npc { spawn, .. }) = instance.find_actor_mut(*actor_id) {
                    spawn.common.position = *position;
                    spawn.common.rotation = *rotation;
                }
                let mut network = network.lock();
                network.send_in_range_inclusive_instance(
                    *actor_id,
                    instance,
                    FromServer::ActorMove(
                        *actor_id,
                        *position,
                        *rotation,
                        MoveAnimationType::empty(),
                        MoveAnimationState::empty(),
                        JumpState::empty(),
                    ),
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::SpawnClone {
                base_id,
                name_id,
                position,
                rotation,
            } => {
                let bnpc = {
                    let mut game_data = gamedata.lock();
                    game_data.find_bnpc(*base_id).map(
                        |(model_chara, battalion, customize, _rank, equip)| {
                            let equip_spawn =
                                game_data.get_npc_equip(equip as u32).unwrap_or_default();
                            (model_chara, battalion, customize, equip_spawn)
                        },
                    )
                };
                if let Some((model_chara, battalion, customize, equip_spawn)) = bnpc {
                    let actor_id = ObjectId(fastrand::u32(..));
                    // Non-hostile + AI-paused clone: it never fights, the script drives it. Nameplate
                    // hidden via UNK1|UNK2 (same as the helper pool); the MODEL is hidden/shown via
                    // ActorControl 414 (the client's bit16) — clones spawn `visible = false` so the
                    // walked-in path sends that 414, and `set_visible(true)` shows it for the charge.
                    let npc = SpawnNpc {
                        common: CommonSpawn {
                            handler_id: director_id,
                            base_id: *base_id,
                            name_id: *name_id,
                            health_points: 1,
                            max_health_points: 1,
                            display_flags: DisplayFlag::UNK1 | DisplayFlag::UNK2,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            battalion,
                            model_chara,
                            position: *position,
                            rotation: *rotation,
                            look: customize,
                            ..equip_spawn
                        },
                        ..Default::default()
                    };
                    instance.insert_npc(actor_id, npc);
                    if let Some(NetworkedActor::Npc {
                        state,
                        ai_paused,
                        targetable,
                        visible,
                        ..
                    }) = instance.find_actor_mut(actor_id)
                    {
                        *state = NpcState::Stay;
                        *ai_paused = true;
                        *targetable = false; // visual-only; re-applied on each walked-in spawn
                        // Born hidden: clones idle on the arena edge and are only shown during their
                        // mechanic. Re-applied on walked-in so a fresh spawn never pops into view.
                        *visible = false;
                    }
                    if let Some(director) = &mut instance.director {
                        director.clones.push(actor_id);
                    }
                } else {
                    tracing::warn!("Failed to find BNpcBase {base_id} for SpawnClone");
                }
            }
        }
    }

    if let Some(director) = &mut instance.director {
        // Remove only the tasks we actually processed (the cloned prefix). A handler can push *new*
        // tasks while we iterate — e.g. CompleteDuty → stop_timeline queues DespawnActor for the
        // helper/clone pools — and those must survive to be run next tick, not be cleared here.
        let processed = tasks.len().min(director.tasks.len());
        director.tasks.drain(0..processed);
        director.bosses = bosses;
    }

    pending_aoes
}

/// Resolve a pending AoE at its precise activation time: snapshot each alive player's *current*
/// position, hit those inside the shape for flat `damage`, push a damage number + enmity, and update
/// HP. Called from a precise tokio timer (not the 8Hz tick) so the snapshot lands exactly on time —
/// a player who walked out before activation is spared, matching retail's reaction window.
pub fn resolve_aoe(network: Arc<Mutex<NetworkState>>, instance: &mut Instance, aoe: PendingAoe) {
    // The action has resolved, so release the caster's cast-lock (set by CastBar). This fires for
    // the cast's own completing effect too (cast() schedules a radius-0 resolve at cast end), so the
    // boss returns to chasing/auto-attacking exactly when the cast bar finishes.
    if let Some(NetworkedActor::Npc { cast_locked, .. }) = instance.find_actor_mut(aoe.source_id) {
        *cast_locked = false;
    }

    // Find every alive player caught in the shape, reading positions *now*.
    let origin = aoe.origin.0;
    let hit_players: Vec<ObjectId> = instance
        .actors
        .iter()
        .filter_map(|(id, actor)| match actor {
            NetworkedActor::Player { spawn, .. }
                if spawn.common.health_points > 0
                    && aoe
                        .shape
                        .contains(spawn.common.position.0, origin, aoe.rotation) =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();

    // DEBUG: show whether players are actually inside the shape at resolve time. If you're standing
    // in the AoE but `hit=0`, the server's idea of your position differs from where the omen rendered
    // (a position desync / wrong origin), not a packet issue.
    {
        let detail: Vec<String> = instance
            .actors
            .iter()
            .filter_map(|(id, a)| match a {
                NetworkedActor::Player { spawn, .. } if spawn.common.health_points > 0 => {
                    let p = spawn.common.position.0;
                    let d = ((p.x - origin.x).powi(2) + (p.z - origin.z).powi(2)).sqrt();
                    Some(format!(
                        "{:?}@({:.1},{:.1}) dist={:.1} in={}",
                        id,
                        p.x,
                        p.z,
                        d,
                        aoe.shape.contains(p, origin, aoe.rotation)
                    ))
                }
                _ => None,
            })
            .collect();
        tracing::info!(
            "[resolve_aoe] action={} shape={:?} origin=({:.1},{:.1}) rot={:.2} hit={} | {}",
            aoe.action_id,
            aoe.shape,
            origin.x,
            origin.z,
            aoe.rotation,
            hit_players.len(),
            detail.join(", ")
        );
    }

    if hit_players.is_empty() && aoe.action_id == 0 {
        return;
    }

    // Apply damage and accumulate enmity onto the source, building the per-target effect list.
    let mut targets: Vec<(ObjectTypeId, ActionEffect)> = Vec::new();
    for target_id in &hit_players {
        let effect = ActionEffect {
            kind: EffectKind::Damage {
                damage_kind: DamageKind::Normal,
                damage_type: DamageType::Magic,
                damage_element: DamageElement::Unaspected,
                bonus_percent: 0,
                unk3: 0,
                unk4: 0,
                amount: aoe.damage,
            },
        };

        if let Some(target) = instance.find_actor_mut(*target_id) {
            target.apply_damage(aoe.damage);
        }

        // Give the source enmity for the hit, so it keeps aggro through mechanic damage.
        if let Some(source) = instance.find_actor_mut(aoe.source_id)
            && let Some(hate_list) = source.npc_hate_list_mut()
        {
            let entry = hate_list.entry(*target_id).or_insert(0);
            *entry = entry.saturating_add(aoe.damage.max(1));
        }

        targets.push((
            ObjectTypeId {
                object_id: *target_id,
                object_type: ObjectTypeKind::None,
            },
            effect,
        ));
    }

    // Allocate the global action sequence up front — both the effect packet and its per-target
    // `EffectResultBasic` confirmation must carry the SAME value, or the client can't link them.
    let global_sequence = {
        let mut network = network.lock();
        let seq = network.global_action_sequence;
        network.global_action_sequence += 1;
        seq
    };
    let anim_source = aoe.effect_source.unwrap_or(aoe.source_id);

    // Send the result as an `AoeEffect8` — it's the only result packet with a `position` field, so
    // the burst renders at the ground AoE centre (`aoe.origin`) regardless of the animation target.
    // Sourced from the off-arena omen helper so charge actions don't drag the boss. retail uses
    // `animation_target = no-target` even for a HIT (verified vs a captured Eruption: AoeEffect8 with
    // animation_target 0xE0000000, the damage in the per-target effects, and the player confirmed by
    // a following EffectResultBasic). 0 hits also uses no-target — the position field draws the burst.
    {
        let mut network = network.lock();
        let capped = &targets[..targets.len().min(8)];
        let mut effects = [[ActionEffect::default(); 8]; 8];
        let mut target_ids = [ObjectTypeId::default(); 8];
        for (i, (target, effect)) in capped.iter().enumerate() {
            effects[i][0] = *effect;
            target_ids[i] = *target;
        }

        let header = AoeEffectHeader {
            animation_target_id: ObjectTypeId::default(), // no-target (0xE0000000), like retail
            action_id: aoe.action_id,
            global_sequence,
            animation_lock: 0.6,
            spell_id: aoe.action_id as u16,
            rotation: aoe.rotation,
            action_type: ActionType::Action,
            target_count: capped.len() as u8,
            ..Default::default()
        };

        let ipc = ServerZoneIpcData::AoeEffect8(Box::new(AoeEffect8 {
            header,
            effects,
            target_ids,
            position: aoe.origin,
        }));

        network.send_in_range_inclusive_instance(
            anim_source,
            instance,
            FromServer::PacketSegment(ServerZoneIpcSegment::new(ipc), anim_source),
            DestinationNetwork::ZoneClients,
        );
    }

    // Retail order (verified vs capture): effect → **EffectResultBasic** → HP update. The
    // `EffectResultBasic` (opcode 0x02FC, 24 bytes — NOT the 96-byte status `EffectResult`) is what
    // confirms the hit: its `unk2` is the action's `global_sequence`, and the client matches that to
    // the effect packet to mark the target confirmed. Without a matching sequence the client renders
    // the effect as a **miss** even though HP changed (symptom: "HP 被减了但客户端显示失误").
    for target_id in hit_players {
        if let Some(actor) = instance.find_actor(target_id) {
            let current_hp = actor.get_common_spawn().health_points;
            let ipc =
                ServerZoneIpcSegment::new(ServerZoneIpcData::EffectResultBasic {
                    unk1: 1,
                    unk2: global_sequence,
                    target_id,
                    current_hp,
                    unk3: 0,
                    unk4: 0,
                });
            network.lock().send_in_range_inclusive_instance(
                target_id,
                instance,
                FromServer::PacketSegment(ipc, target_id),
                DestinationNetwork::ZoneClients,
            );
        }
        update_actor_hp_mp(network.clone(), instance, target_id);
    }
}

/// Process director-related messages.
pub fn handle_director_messages(data: Arc<Mutex<WorldServer>>, msg: &ToServer) -> bool {
    match msg {
        ToServer::GimmickAccessor(from_actor_id, from_object_id, params) => {
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(*from_actor_id) else {
                tracing::warn!("Somehow failed to find an instance for actor?");
                return true;
            };

            let Some(id) = instance.find_base_id_by_actor_id(*from_object_id) else {
                tracing::warn!("Somehow failed to find base id from actor id {from_object_id}!");
                return true;
            };

            if let Some(director) = &mut instance.director {
                director.gimmick_accessor(*from_actor_id, id, params);
            } else {
                tracing::warn!("Expected a director when recieving a GimmickAccessor?");
            }

            true
        }
        ToServer::VariantVote(from_actor_id, vote) => {
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(*from_actor_id) else {
                tracing::warn!("Somehow failed to find an instance for actor?");
                return true;
            };

            if let Some(director) = &mut instance.director {
                director.variant_vote(*vote);
            } else {
                tracing::warn!("Expected a director when recieving a VariantVote?");
            }

            true
        }
        _ => false,
    }
}
