//! Managing statistics, including your classjob and other related information.

use crate::{GameData, ZoneConnection};
use kawari::{
    common::{MAXIMUM_MP, MAXIMUM_RESTED_EXP, ObjectId},
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, PlayerStats, ServerZoneIpcData,
        ServerZoneIpcSegment, UpdateClassInfo,
    },
    packet::{PacketSegment, SegmentData, SegmentType},
};

impl ZoneConnection {
    pub async fn update_class_info(&mut self) {
        let ipc;
        {
            let game_data = self.gamedata.lock();

            ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo {
                class_id: self.player_data.classjob_id,
                class_level: self.current_level(&game_data),
                current_level: self.current_level(&game_data),
                current_exp: self.current_exp(&game_data),
                ..Default::default()
            }));
        }
        self.send_ipc_self(ipc).await;
    }

    pub async fn send_stats(&mut self) {
        let attributes;
        {
            let mut game_data = self.gamedata.lock();

            attributes = game_data
                .get_racial_base_attributes(self.player_data.subrace)
                .expect("Failed to read racial attributes");
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerStats(PlayerStats {
            strength: attributes.strength,
            dexterity: attributes.dexterity,
            vitality: attributes.vitality,
            intelligence: attributes.intelligence,
            mind: attributes.mind,
            hp: 1000, // TODO: hardcoded
            mp: MAXIMUM_MP as u32,
            ..Default::default()
        }));
        self.send_ipc_self(ipc).await;
    }

    pub fn current_level(&self, game_data: &GameData) -> u16 {
        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_levels[index as usize]
    }

    pub fn set_current_level(&mut self, level: u16) {
        self.set_level_for(self.player_data.classjob_id, level);
    }

    pub fn set_level_for(&mut self, classjob_id: u8, level: u16) {
        let game_data = self.gamedata.lock();

        let index = game_data.get_exp_array_index(classjob_id as u16).unwrap();
        self.player_data.classjob_levels[index as usize] = level;
    }

    pub fn current_exp(&self, game_data: &GameData) -> i32 {
        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_exp[index as usize]
    }

    pub fn set_current_exp(&mut self, exp: i32) {
        let game_data = self.gamedata.lock();

        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_exp[index as usize] = exp;
    }

    pub async fn update_hp_mp(&mut self, actor_id: ObjectId, hp: u32, mp: u16) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateHpMpTp { hp, mp, unk: 0 });

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    /// Adds EXP to the current classjob, handles level-up and so on.
    pub async fn add_exp(&mut self, exp: i32) {
        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::EXPFloatingMessage {
                classjob_id: self.player_data.classjob_id as u32,
                amount: exp as u32,
                unk2: 0,
                unk3: 0,
            },
        })
        .await;

        let index;
        let mut level_up = 0;
        {
            let mut game_data = self.gamedata.lock();

            index = game_data
                .get_exp_array_index(self.player_data.classjob_id as u16)
                .unwrap();

            self.player_data.classjob_exp[index as usize] += exp;

            // Keep going until we have leftover EXP
            loop {
                let curr_exp = self.player_data.classjob_exp[index as usize];
                let max_exp =
                    game_data.get_max_exp(self.player_data.classjob_levels[index as usize] as u32);
                let difference = curr_exp - max_exp;
                if difference >= 0 {
                    level_up += 1;
                    self.player_data.classjob_exp[index as usize] = difference;
                } else {
                    break;
                }
            }
        }

        if level_up > 0 {
            let curr_level = self.player_data.classjob_levels[index as usize];
            let new_level = curr_level + level_up;
            self.set_current_level(new_level);

            self.actor_control_self(ActorControlSelf {
                category: ActorControlCategory::LevelUpMessage {
                    classjob_id: self.player_data.classjob_id as u32,
                    level: new_level as u32,
                    unk2: 0,
                    unk3: 0,
                },
            })
            .await;
        }

        // TODO: re-send stats like in retail

        self.update_class_info().await;
    }

    /// The number of seconds to add to the rested EXP bonus.
    pub async fn add_rested_exp_seconds(&mut self, seconds: i32) {
        self.player_data.rested_exp += seconds;
        self.player_data.rested_exp = self.player_data.rested_exp.clamp(0, MAXIMUM_RESTED_EXP);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::UpdateRestedExp {
                exp: self.player_data.rested_exp as u32,
            },
        })
        .await;
    }
}
