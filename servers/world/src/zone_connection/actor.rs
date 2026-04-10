//! Everything to do with spawning, managing and moving actors - including the player.

use crate::{ToServer, ZoneConnection, common::SpawnKind};
use kawari::{
    common::{
        CharacterMode, EquipDisplayFlag, JumpState, MoveAnimationState, MoveAnimationType,
        ObjectId, ObjectTypeId, Position,
    },
    config::get_config,
    ipc::zone::{
        ActorControl, ActorControlCategory, ActorControlSelf, ActorControlTarget, ActorMove,
        CommonSpawn, Config, DisplayFlag, ObjectKind, PlayerSubKind, ServerZoneIpcData,
        ServerZoneIpcSegment, SpawnObject, SpawnPlayer, SpawnTreasure, Warp,
    },
};

impl ZoneConnection {
    /// Sets the player new position and rotation. Must be a location within the current zone.
    pub async fn set_player_position(&mut self, position: Position, rotation: f32, fade_out: bool) {
        // set pos
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Warp(Warp {
                position,
                dir: rotation,
                warp_type: if fade_out { 1 } else { 0 },
                warp_type_arg: if fade_out { 2 } else { 0 },
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
        }
    }

    pub async fn set_actor_position(
        &mut self,
        actor_id: ObjectId,
        position: Position,
        rotation: f32,
        anim_type: MoveAnimationType,
        anim_state: MoveAnimationState,
        jump_state: JumpState,
    ) {
        const SPEED_WALKING: u8 = 20;
        const SPEED_RUNNING: u8 = 60;

        let mut anim_type = anim_type;
        let mut anim_speed = SPEED_RUNNING; // TODO: sprint is 78, jog is 72, but falling and normal running are always 60

        // We're purely walking or strafing while walking. No jumping or falling.
        if anim_type & MoveAnimationType::WALKING_OR_LANDING
            == MoveAnimationType::WALKING_OR_LANDING
            && anim_state == MoveAnimationState::None
            && jump_state == JumpState::NoneOrFalling
        {
            anim_speed = SPEED_WALKING;
        }

        if anim_state == MoveAnimationState::LeavingCollision {
            anim_type |= MoveAnimationType::FALLING;
        }

        if jump_state == JumpState::Ascending {
            anim_type |= MoveAnimationType::FALLING;
            if anim_state == MoveAnimationState::LeavingCollision
                || anim_state == MoveAnimationState::StartFalling
            {
                anim_type |= MoveAnimationType::JUMPING;
            }
        }

        if anim_state == MoveAnimationState::EnteringCollision {
            anim_type = MoveAnimationType::WALKING_OR_LANDING;
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorMove(ActorMove {
            rotation,

            anim_type,
            anim_state,
            anim_speed,
            position,
        }));

        self.send_ipc_from(actor_id, ipc).await;
    }

    pub async fn spawn_actor(&mut self, actor_id: ObjectId, spawn: SpawnKind) {
        // There is no reason for us to spawn our own player again. It's probably a bug!
        assert!(actor_id != self.player_data.character.actor_id);

        let ipc = match spawn {
            SpawnKind::Player(spawn) => {
                ServerZoneIpcSegment::new(ServerZoneIpcData::SpawnPlayer(spawn))
            }
            SpawnKind::Npc(spawn) => ServerZoneIpcSegment::new(ServerZoneIpcData::SpawnNpc(spawn)),
        };
        self.send_ipc_from(actor_id, ipc).await;
    }

    pub async fn delete_actor(&mut self, actor_id: ObjectId, spawn_index: u8) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::DeleteActor {
            spawn_index,
            actor_id,
        });

        self.send_ipc_from(actor_id, ipc).await;
    }

    pub async fn delete_object(&mut self, spawn_index: u8) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::DeleteObject { spawn_index });

        self.send_ipc_self(ipc).await;
    }

    pub async fn toggle_invisibility(&mut self, invisible: bool) {
        self.player_data.gm_invisible = invisible;
        self.actor_control_self(ActorControlCategory::ToggleInvisibility { invisible })
            .await;
    }

    pub async fn actor_control_self(&mut self, category: ActorControlCategory) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category,
            }));
        self.send_ipc_self(ipc).await;
    }

    /// Broadcasts an actor control to everyone around you, including yourself. Useful for stuff like crafting.
    pub async fn broadcast_actor_control(&mut self, category: ActorControlCategory) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: category.clone(),
            }));
        self.send_ipc_self(ipc).await;

        self.handle
            .send(ToServer::BroadcastActorControl(
                self.player_data.character.actor_id,
                category,
            ))
            .await;
    }

    pub async fn actor_control(&mut self, actor_id: ObjectId, category: ActorControlCategory) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControl(ActorControl { category }));

        self.send_ipc_from(actor_id, ipc).await;
    }

    pub async fn actor_control_target(
        &mut self,
        actor_id: ObjectId,
        target: ObjectTypeId,
        category: ActorControlCategory,
    ) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlTarget(ActorControlTarget {
                category,
                target,
            }));

        self.send_ipc_from(actor_id, ipc).await;
    }

    /// Spawn the player actor. The client will handle replacing the existing one, if it exists.
    pub async fn respawn_player(&mut self, start_invisible: bool) -> SpawnPlayer {
        let common = self.get_player_common_spawn(start_invisible);
        let config = get_config();

        let spawn = SpawnPlayer {
            account_id: self.player_data.character.service_account_id as u64,
            content_id: self.player_data.character.content_id as u64,
            current_world_id: config.world.world_id,
            home_world_id: config.world.world_id,
            gm_rank: self.player_data.character.gm_rank,
            online_status: self.get_actual_online_status(),
            common: common.clone(),
            title_id: self.player_data.volatile.title as u16,
            ..Default::default()
        };

        // send player spawn
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SpawnPlayer(spawn.clone()));
            self.send_ipc_self(ipc).await;
        }

        self.spawned_in = true;

        spawn
    }

    pub async fn update_config(&mut self, actor_id: ObjectId, config: Config) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Config(config));

        self.send_ipc_from(actor_id, ipc).await;
    }

    fn get_player_common_spawn(&self, start_invisible: bool) -> CommonSpawn {
        let inventory = &self.player_data.inventory;

        let mut database = self.database.lock();
        let chara_make = database.get_chara_make(self.player_data.character.content_id as u64);
        let mut look = chara_make.customize;

        // There seems to be no display flag for this, so clear the bit out
        if self
            .player_data
            .volatile
            .display_flags
            .intersects(EquipDisplayFlag::HIDE_LEGACY_MARK)
        {
            look.facial_features &= !(1 << 7);
        }

        let mut display_flags = self.player_data.volatile.display_flags.into();
        if start_invisible {
            display_flags |= DisplayFlag::INVISIBLE;
        }

        let base_parameters = self.base_parameters(); // TODO: maybe cache this?
        let mut game_data = self.gamedata.lock();

        CommonSpawn {
            class_job: self.player_data.classjob.current_class as u8,
            name: self.player_data.character.name.clone(),
            health_points: base_parameters.hp,
            max_health_points: base_parameters.hp,
            resource_points: base_parameters.mp as u16,
            max_resource_points: base_parameters.mp as u16,
            level: self.current_level(&game_data) as u8,
            object_kind: ObjectKind::Player(PlayerSubKind::Player),
            look,
            display_flags,
            main_weapon_model: inventory.get_main_weapon_id(&mut game_data),
            sec_weapon_model: inventory.get_sub_weapon_id(&mut game_data),
            models: inventory.get_model_ids(&mut game_data),
            position: self.player_data.volatile.position,
            rotation: self.player_data.volatile.rotation as f32,
            voice: chara_make.voice_id as u8,
            active_minion: self.active_minion as u16,
            handler_id: self.content_handler_id,
            // TODO: Dismount if entering a duty? Towns are probably fine to leave alone.
            current_mount: self.player_data.volatile.current_mount as u16,
            mode: if self.player_data.volatile.current_mount != 0 {
                CharacterMode::Mounted
            } else {
                CharacterMode::default()
            },
            ..Default::default()
        }
    }

    pub async fn send_conditions(&mut self) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Condition(self.conditions));
        self.send_ipc_self(ipc).await;

        // Inform the server state as well
        self.handle
            .send(ToServer::UpdateConditions(
                self.player_data.character.actor_id,
                self.conditions,
            ))
            .await;
    }

    pub async fn spawn_object(&mut self, spawn: SpawnObject) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SpawnObject(spawn));

        self.send_ipc_from(spawn.entity_id, ipc).await;
    }

    /// Sets this actor's CharacterMode and informs other clients.
    pub async fn set_character_mode(&mut self, mode: CharacterMode, arg: u8) {
        self.handle
            .send(ToServer::SetCharacterMode(
                self.player_data.character.actor_id,
                mode,
                arg,
            ))
            .await;
    }

    pub async fn spawn_treasure(&mut self, spawn: SpawnTreasure) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SpawnTreasure(spawn.clone()));

        self.send_ipc_from(spawn.entity_id, ipc).await;
    }
}
