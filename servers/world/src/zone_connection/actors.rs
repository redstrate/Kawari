//! Everything to do with spawning, managing and moving actors - including the player.

use crate::{ToServer, ZoneConnection, common::SpawnKind};
use kawari::{
    common::{
        EquipDisplayFlag, JumpState, MAXIMUM_MP, MoveAnimationSpeed, MoveAnimationState,
        MoveAnimationType, ObjectId, ObjectTypeId, ObjectTypeKind, Position,
    },
    config::get_config,
    ipc::zone::{
        ActorControl, ActorControlCategory, ActorControlSelf, ActorControlTarget, ActorMove,
        CommonSpawn, Config, DisplayFlag, GameMasterRank, ObjectKind, ObjectSpawn, OnlineStatus,
        PlayerSpawn, PlayerSubKind, ServerZoneIpcData, ServerZoneIpcSegment, Warp,
    },
    packet::{PacketSegment, SegmentData, SegmentType},
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
        let mut anim_type = anim_type;
        let mut anim_speed = MoveAnimationSpeed::Running; // TODO: sprint is 78, jog is 72, but falling and normal running are always 60

        // We're purely walking or strafing while walking. No jumping or falling.
        if anim_type & MoveAnimationType::WALKING_OR_LANDING
            == MoveAnimationType::WALKING_OR_LANDING
            && anim_state == MoveAnimationState::None
            && jump_state == JumpState::NoneOrFalling
        {
            anim_speed = MoveAnimationSpeed::Walking;
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

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    pub async fn spawn_actor(&mut self, actor_id: ObjectId, spawn: SpawnKind) {
        // There is no reason for us to spawn our own player again. It's probably a bug!
        assert!(actor_id != self.player_data.actor_id);

        let ipc;

        // TODO: Can this be deduplicated somehow?
        match spawn {
            SpawnKind::Player(mut spawn) => {
                spawn.common.target_id = ObjectTypeId {
                    object_id: actor_id,
                    object_type: ObjectTypeKind::None,
                };
                ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerSpawn(spawn));
            }
            SpawnKind::Npc(mut spawn) => {
                spawn.common.target_id = ObjectTypeId {
                    object_id: actor_id,
                    object_type: ObjectTypeKind::None,
                };
                ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::NpcSpawn(spawn));
            }
        }

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;

        self.actor_control(
            actor_id,
            ActorControl {
                category: ActorControlCategory::ZoneIn {
                    warp_finish_anim: 1,
                    raise_anim: 0,
                    unk1: 0,
                },
            },
        )
        .await;
    }

    pub async fn delete_actor(&mut self, actor_id: ObjectId, spawn_index: u8) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::DeleteActor {
            spawn_index,
            actor_id,
        });

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    pub async fn delete_object(&mut self, spawn_index: u8) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::DeleteObject { spawn_index });

        self.send_segment(PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    pub async fn toggle_invisibility(&mut self, invisible: bool) {
        self.player_data.gm_invisible = invisible;
        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleInvisibility { invisible },
        })
        .await;
    }

    pub async fn actor_control_self(&mut self, actor_control: ActorControlSelf) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(actor_control));
        self.send_ipc_self(ipc).await;
    }

    pub async fn actor_control(&mut self, actor_id: ObjectId, actor_control: ActorControl) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControl(actor_control));

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    pub async fn actor_control_target(
        &mut self,
        actor_id: ObjectId,
        actor_control: ActorControlTarget,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlTarget(actor_control));

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    /// Spawn the player actor. The client will handle replacing the existing one, if it exists.
    pub async fn respawn_player(&mut self, start_invisible: bool) -> PlayerSpawn {
        let common =
            self.get_player_common_spawn(self.exit_position, self.exit_rotation, start_invisible);
        let config = get_config();

        let online_status = if self.player_data.gm_rank == GameMasterRank::NormalUser {
            OnlineStatus::Online
        } else {
            OnlineStatus::GameMasterBlue
        };

        let spawn = PlayerSpawn {
            account_id: self.player_data.account_id,
            content_id: self.player_data.content_id,
            current_world_id: config.world.world_id,
            home_world_id: config.world.world_id,
            gm_rank: self.player_data.gm_rank,
            online_status,
            common: common.clone(),
            title_id: self.player_data.title,
            ..Default::default()
        };

        // send player spawn
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerSpawn(spawn.clone()));
            self.send_ipc_self(ipc).await;
        }

        spawn
    }

    pub async fn update_config(&mut self, actor_id: ObjectId, config: Config) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Config(config));

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    pub fn get_player_common_spawn(
        &self,
        exit_position: Option<Position>,
        exit_rotation: Option<f32>,
        start_invisible: bool,
    ) -> CommonSpawn {
        let mut game_data = self.gamedata.lock();

        let inventory = &self.player_data.inventory;

        let mut database = self.database.lock();
        let chara_make = database.get_chara_make(self.player_data.content_id);
        let mut look = chara_make.customize;

        // There seems to be no display flag for this, so clear the bit out
        if self
            .player_data
            .display_flags
            .intersects(EquipDisplayFlag::HIDE_LEGACY_MARK)
        {
            look.facial_features &= !(1 << 7);
        }

        let mut display_flags = self.player_data.display_flags.into();
        if start_invisible {
            display_flags |= DisplayFlag::INVISIBLE;
        }

        CommonSpawn {
            class_job: self.player_data.classjob_id,
            name: self.player_data.name.clone(),
            hp_curr: 1000, // TODO: hardcoded
            hp_max: 1000,
            mp_curr: MAXIMUM_MP,
            mp_max: MAXIMUM_MP,
            level: self.current_level(&game_data) as u8,
            object_kind: ObjectKind::Player(PlayerSubKind::Player),
            look,
            display_flags,
            main_weapon_model: inventory.get_main_weapon_id(&mut game_data),
            sec_weapon_model: inventory.get_sub_weapon_id(&mut game_data),
            models: inventory.get_model_ids(&mut game_data),
            pos: exit_position.unwrap_or_default(),
            rotation: exit_rotation.unwrap_or(0.0),
            voice: chara_make.voice_id as u8,
            active_minion: self.player_data.active_minion as u16,
            ..Default::default()
        }
    }

    pub async fn send_conditions(&mut self) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Condition(self.conditions));
        self.send_ipc_self(ipc).await;

        // Inform the server state as well
        self.handle
            .send(ToServer::UpdateConditions(
                self.player_data.actor_id,
                self.conditions,
            ))
            .await;
    }

    pub async fn spawn_object(&mut self, spawn: ObjectSpawn) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ObjectSpawn(spawn));

        self.send_segment(PacketSegment {
            source_actor: spawn.entity_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }
}
