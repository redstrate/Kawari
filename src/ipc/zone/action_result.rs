use binrw::binrw;
use serde::{Deserialize, Serialize};

use crate::common::{ObjectId, ObjectTypeId, read_quantized_rotation, write_quantized_rotation};

// TODO: this might be a flag?
#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, Copy, Default, Deserialize, Serialize)]
#[brw(repr = u8)]
pub enum DamageKind {
    #[default]
    Normal = 0x0,
    Critical = 0x1,
    DirectHit = 0x2,
}

#[binrw]
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum EffectKind {
    #[default]
    #[brw(magic = 0u8)]
    Miss, // FIXME: is this correct?
    #[brw(magic = 3u8)]
    Damage {
        damage_kind: DamageKind,
        #[br(temp)]
        #[bw(calc = 0)]
        param1: u8,
        #[br(calc = DamageType::from(param1 & 0x0F))]
        #[bw(ignore)]
        damage_type: DamageType,
        #[br(calc = DamageElement::from(param1 >> 4))]
        #[bw(ignore)]
        damage_element: DamageElement,
        bonus_percent: u8,
        unk3: u8,
        unk4: u8,
        amount: u16,
    },
    #[brw(magic = 27u8)]
    BeginCombo,
    /// seen during sprint
    #[brw(magic = 14u8)]
    Unk1 {
        unk1: u8,
        unk2: u32,
        effect_id: u16,

        // NOTE: the following is for our internal usage, this is not an actual part of the packet
        // TODO: this shouldn't be here, instead we should maybe create a lua-specific struct for all of this information
        #[brw(ignore)]
        duration: f32,
        #[brw(ignore)]
        param: u16,
        #[brw(ignore)]
        source_actor_id: ObjectId,
    },
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Default, Deserialize, Serialize)]
pub enum DamageType {
    Unknown,
    Slashing,
    Piercing,
    Blunt,
    Shot,
    Magic,
    Breath,
    #[default]
    Physical,
    LimitBreak,
}

impl From<u8> for DamageType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Slashing,
            2 => Self::Piercing,
            3 => Self::Blunt,
            4 => Self::Shot,
            5 => Self::Magic,
            6 => Self::Breath,
            7 => Self::Physical,
            8 => Self::LimitBreak,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Default, Deserialize, Serialize)]
pub enum DamageElement {
    Unknown,
    Fire,
    Ice,
    Air,
    Earth,
    Lightning,
    Water,
    #[default]
    Unaspected,
}

impl From<u8> for DamageElement {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Fire,
            2 => Self::Ice,
            3 => Self::Air,
            4 => Self::Earth,
            5 => Self::Lightning,
            6 => Self::Water,
            7 => Self::Unaspected,
            _ => Self::Unknown,
        }
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionEffect {
    #[brw(pad_size_to = 8)]
    pub kind: EffectKind,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct ActionResult {
    pub main_target: ObjectTypeId,
    pub action_id: u32,
    pub unk1: u32,
    pub animation_lock_time: f32,
    pub unk2: u32,
    pub hidden_animation: u16,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    pub action_animation_id: u16,
    pub variation: u8,
    pub flag: u8,
    pub unk3: u8,
    pub effect_count: u8,
    pub unk4: u16,
    pub unk5: [u8; 6],
    pub effects: [ActionEffect; 8],
    #[brw(pad_before = 6, pad_after = 4)]
    pub target_id_again: ObjectTypeId,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::common::ObjectId;

    use super::*;

    #[test]
    fn read_actionresult() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/action_result.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_result = ActionResult::read_le(&mut buffer).unwrap();
        assert_eq!(action_result.main_target.object_id, ObjectId(0x40070E42));
        assert_eq!(action_result.action_id, 31);
        assert_eq!(action_result.unk1, 2662353); // TODO: probably means this field is wrong
        assert_eq!(action_result.animation_lock_time, 0.6);
        assert_eq!(action_result.unk2, 3758096384); // TODO: ditto
        assert_eq!(action_result.hidden_animation, 1);
        assert_eq!(action_result.rotation, 1.207309);
        assert_eq!(action_result.action_animation_id, 31);
        assert_eq!(action_result.variation, 0);
        assert_eq!(action_result.flag, 1);
        assert_eq!(action_result.unk3, 0);
        assert_eq!(action_result.effect_count, 1);
        assert_eq!(action_result.unk4, 0);
        assert_eq!(action_result.unk5, [0; 6]);

        // effect 0: attack
        assert_eq!(
            action_result.effects[0].kind,
            EffectKind::Damage {
                damage_kind: DamageKind::Normal,
                damage_type: DamageType::Slashing,
                damage_element: DamageElement::Unaspected,
                bonus_percent: 0,
                unk3: 0,
                unk4: 0,
                amount: 22
            }
        );

        // effect 1: start action combo
        assert_eq!(action_result.effects[1].kind, EffectKind::BeginCombo);

        assert_eq!(
            action_result.target_id_again.object_id,
            ObjectId(0x40070E42)
        );
    }

    #[test]
    fn read_actionresult_sprint() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/action_result_sprint.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_result = ActionResult::read_le(&mut buffer).unwrap();
        assert_eq!(action_result.main_target.object_id, ObjectId(277554542));
        assert_eq!(action_result.action_id, 3);
        assert_eq!(action_result.unk1, 776386); // TODO: probably means this field is wrong
        assert_eq!(action_result.animation_lock_time, 0.6);
        assert_eq!(action_result.unk2, 3758096384); // TODO: ditto
        assert_eq!(action_result.hidden_animation, 1);
        assert_eq!(action_result.rotation, 2.6254003);
        assert_eq!(action_result.action_animation_id, 3);
        assert_eq!(action_result.variation, 0);
        assert_eq!(action_result.flag, 1);
        assert_eq!(action_result.unk3, 0);
        assert_eq!(action_result.effect_count, 1);
        assert_eq!(action_result.unk4, 0);
        assert_eq!(action_result.unk5, [0; 6]);

        // effect 0: unk
        assert_eq!(
            action_result.effects[0].kind,
            EffectKind::Unk1 {
                unk1: 0,
                unk2: 7728,
                effect_id: 50,
                duration: 0.0,
                param: 0,
            }
        );

        assert_eq!(action_result.target_id_again.object_id, ObjectId(277554542));
    }
}
