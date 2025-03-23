use binrw::binrw;

use crate::common::{CHAR_NAME_MAX_LENGTH, CustomizeData, Position, read_string, write_string};

use super::StatusEffect;

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, PartialEq, Debug, Default)]
pub enum ObjectKind {
    #[default]
    None = 0,
    Player = 1,
    BattleNpc = 2,
    EventNpc = 3,
    Treasure = 4,
    Aetheryte = 5,
    GatheringPoint = 6,
    EventObj = 7,
    Mount = 8,
    Companion = 9,
    Retainer = 10,
    AreaObject = 11,
    HousingEventObject = 12,
    Cutscene = 13,
    MjiObject = 14,
    Ornament = 15,
    CardStand = 16,
}

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
pub struct CommonSpawn {
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
    pub bnpc_base: u32, // See BNpcBase Excel sheet
    pub bnpc_name: u32, // See BNpcName Excel sheet
    pub unk3: [u8; 8],
    pub director_id: u32, // FIXME: i think the next three are in the wrong order
    pub spawner_id: u32,
    pub parent_actor_id: u32,
    pub hp_max: u32,
    pub hp_curr: u32,
    pub display_flags: u32, // assumed
    pub fate_id: u16,       // assumed
    pub mp_curr: u16,
    pub mp_max: u16,
    pub unk: u16,
    pub model_chara: u16,   // See ModelChara Excel sheet
    pub rotation: u16,      // assumed
    pub current_mount: u16, // assumed
    pub active_minion: u16, // assumed
    pub u23: u8,            // assumed
    pub u24: u8,            // assumed
    pub u25: u8,            // assumed
    pub u26: u8,            // assumed
    pub spawn_index: u8,
    pub mode: CharacterMode,
    pub persistent_emote: u8,
    pub object_kind: ObjectKind,
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
}
