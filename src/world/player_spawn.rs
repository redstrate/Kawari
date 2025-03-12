use binrw::binrw;

use crate::common::{read_string, write_string};

use super::position::Position;
use super::status_effect::StatusEffect;

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct PlayerSpawn {
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
    pub padding4: [u8; 16],
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
    pub unk30: u8,
    pub mount_head: u8,
    pub mount_body: u8,
    pub mount_feet: u8,
    pub mount_color: u8,
    pub scale: u8,
    pub element_data: [u8; 6],
    pub padding2: [u8; 12],
    pub effect: [StatusEffect; 30],
    pub pos: Position,
    pub models: [u32; 10],
    pub unknown6_58: [u8; 10],
    pub padding3: [u8; 7],
    #[br(count = 32)]
    #[bw(pad_size_to = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    pub look: [u8; 26],
    pub fc_tag: [u8; 6],
    pub padding: [u8; 26],
}
