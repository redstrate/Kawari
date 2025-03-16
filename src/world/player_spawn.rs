use binrw::binrw;

use crate::CHAR_NAME_MAX_LENGTH;
use crate::client_select_data::ClientCustomizeData;
use crate::common::{read_string, write_string};

use super::position::Position;
use super::status_effect::StatusEffect;

#[binrw]
#[brw(little)]
#[brw(repr = u8)]
#[derive(Debug, Clone, Default, PartialEq)]
pub enum CharacterMode {
    None = 0x0,
    #[default]
    Normal = 0x1,
    Dead = 0x2,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct PlayerSpawn {
    // also shows up in the friends list.
    pub some_unique_id: u32,

    #[brw(pad_before = 4)] // always empty?
    pub content_id: u64,

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
    pub mode: CharacterMode,
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
    pub look: ClientCustomizeData,
    #[br(count = 6)]
    #[bw(pad_size_to = 6)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub fc_tag: String,
    pub padding: [u8; 2],
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use super::*;

    #[test]
    fn read_playerspawn() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/player_spawn.dat");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let player_spawn = PlayerSpawn::read_le(&mut buffer).unwrap();
        assert_eq!(player_spawn.current_world_id, 0x4F);
        assert_eq!(player_spawn.home_world_id, 0x4F);
        assert_eq!(player_spawn.hp_curr, 159);
        assert_eq!(player_spawn.hp_max, 159);
        assert_eq!(player_spawn.mp_curr, 10000);
        assert_eq!(player_spawn.mp_max, 10000);
        assert_eq!(player_spawn.mode, CharacterMode::Normal);
        assert_eq!(player_spawn.spawn_index, 0);
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
        assert_eq!(player_spawn.subtype, 4);
    }
}
