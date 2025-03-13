use binrw::binrw;

use crate::CHAR_NAME_MAX_LENGTH;
use crate::client_select_data::ClientCustomizeData;
use crate::common::{read_string, write_string};

use super::position::Position;
use super::status_effect::StatusEffect;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct PlayerSpawn {
    pub aafafaf: [u8; 16],

    pub title: u16,
    pub u1b: u16,
    pub current_world_id: u16,
    pub home_world_id: u16,

    pub gm_rank: u8,
    pub u3c: u8,
    pub u4: u8,
    pub online_status: u8,

    pub pose: u8,
    pub u5a: u8,
    pub u5b: u8,
    pub u5c: u8,

    pub target_id: u64,
    pub u6: u32,
    pub u7: u32,
    pub main_weapon_model: u64,
    pub sec_weapon_model: u64,
    pub craft_tool_model: u64,

    pub u14: u32,
    pub u15: u32,
    pub b_npc_base: u32,
    pub b_npc_name: u32,
    pub u18: u32,
    pub u19: u32,
    pub director_id: u32,
    pub owner_id: u32,
    pub u22: u32,
    pub hp_max: u32,
    pub hp_curr: u32,
    pub display_flags: u32,
    pub fate_id: u16,
    pub mp_curr: u16,
    pub mp_max: u16,
    pub unk: u16,
    pub model_chara: u16,
    pub rotation: u16,
    pub current_mount: u16,
    pub active_minion: u16,
    pub u23: u8,
    pub u24: u8,
    pub u25: u8,
    pub u26: u8,
    pub spawn_index: u8,
    pub state: u8,
    pub persistent_emote: u8,
    pub model_type: u8,
    pub subtype: u8,
    pub voice: u8,
    pub enemy_type: u8,
    pub unk27: u8,
    pub level: u8,
    pub class_job: u8,
    pub unk28: u8,
    pub unk29: u8,
    pub mount_head: u8,
    pub mount_body: u8,
    pub mount_feet: u8,
    pub mount_color: u8,
    pub scale: u8,
    pub element_data: [u8; 6],
    pub padding2: [u8; 1],
    pub effect: [StatusEffect; 30],
    pub pos: Position,
    pub models: [u32; 10],
    pub unknown6_58: [u8; 10],
    pub padding3: [u8; 4],
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    pub look: CustomizeData,
    #[br(count = 6)]
    #[bw(pad_size_to = 6)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub fc_tag: String,
    pub padding: [u8; 2],
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CustomizeData {
    pub race: u8,
    pub gender: u8,
    pub age: u8,
    pub height: u8,
    pub subrace: u8,
    pub face: u8,
    pub hair: u8,
    pub enable_highlights: u8,
    pub skin_tone: u8,
    pub right_eye_color: u8,
    pub hair_tone: u8,
    pub highlights: u8,
    pub facial_features: u8,
    pub facial_feature_color: u8,
    pub eyebrows: u8,
    pub left_eye_color: u8,
    pub eyes: u8,
    pub nose: u8,
    pub jaw: u8,
    pub mouth: u8,
    pub lips_tone_fur_pattern: u8,
    pub race_feature_size: u8,
    pub race_feature_type: u8,
    pub bust: u8,
    pub face_paint: u8,
    pub face_paint_color: u8,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor};

    use binrw::BinRead;

    use super::*;

    #[test]
    fn read_playerspawn() {
        let buffer = read("/home/josh/Downloads/myfile(1).dat").unwrap();
        let mut buffer = Cursor::new(&buffer);

        let player_spawn = PlayerSpawn::read_le(&mut buffer).unwrap();
        assert_eq!(player_spawn.current_world_id, 0x4F);
        assert_eq!(player_spawn.home_world_id, 0x4F);
        assert_eq!(player_spawn.hp_curr, 159);
        assert_eq!(player_spawn.hp_max, 159);
        assert_eq!(player_spawn.mp_curr, 10000);
        assert_eq!(player_spawn.mp_max, 10000);
        assert_eq!(player_spawn.state, 1);
        assert_eq!(player_spawn.level, 1);
        assert_eq!(player_spawn.class_job, 1); // adventurer
        assert_eq!(player_spawn.scale, 36);
        assert_eq!(player_spawn.pos.x, 40.519722);
        assert_eq!(player_spawn.pos.y, 4.0);
        assert_eq!(player_spawn.pos.z, -150.33124);
        assert_eq!(player_spawn.name, "Lavenaa Warren");
        assert_eq!(player_spawn.look.race, 1);
        assert_eq!(player_spawn.look.gender, 1);
        assert_eq!(player_spawn.look.bust, 100);
        assert_eq!(player_spawn.fc_tag, "");
    }
}
