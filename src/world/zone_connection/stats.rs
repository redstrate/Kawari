//! Managing statistics, including your classjob and other related information.

use crate::{
    common::{GameData, ObjectId},
    ipc::zone::{PlayerStats, ServerZoneIpcData, ServerZoneIpcSegment, UpdateClassInfo},
    packet::{PacketSegment, SegmentData, SegmentType},
    world::{CharacterData, ZoneConnection},
};

impl ZoneConnection {
    pub async fn update_class_info(&mut self) {
        let ipc;
        {
            let game_data = self.gamedata.lock();

            ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo {
                class_id: self.player_data.classjob_id,
                synced_level: self.current_level(&game_data),
                class_level: self.current_level(&game_data),
                current_level: self.current_level(&game_data),
                current_exp: self.current_exp(&game_data),
                ..Default::default()
            }));
        }
        self.send_ipc_self(ipc).await;
    }

    pub async fn send_stats(&mut self, chara_details: &CharacterData) {
        let attributes;
        {
            let mut game_data = self.gamedata.lock();

            attributes = game_data
                .get_racial_base_attributes(chara_details.chara_make.customize.subrace)
                .expect("Failed to read racial attributes");
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerStats(PlayerStats {
            strength: attributes.strength,
            dexterity: attributes.dexterity,
            vitality: attributes.vitality,
            intelligence: attributes.intelligence,
            mind: attributes.mind,
            hp: self.player_data.max_hp,
            mp: self.player_data.max_mp as u32,
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
        let game_data = self.gamedata.lock();

        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
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
            source_actor: actor_id.0,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }
}
