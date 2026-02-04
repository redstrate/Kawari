use binrw::binrw;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, FromRepr};

use crate::common::{ObjectId, ObjectTypeId, read_quantized_rotation, write_quantized_rotation};

// TODO: this might be a flag?
#[binrw]
#[derive(
    Debug, Eq, PartialEq, Clone, Copy, Default, Display, Deserialize, Serialize, EnumIter, FromRepr,
)]
#[repr(u8)]
#[brw(repr = u8)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum DamageKind {
    #[default]
    Normal = 0x0,
    Critical = 0x1,
    DirectHit = 0x2,
}

impl mlua::IntoLua for DamageKind {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

impl mlua::FromLua for DamageKind {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Integer(integer) => Ok(Self::from_repr(integer as u8).unwrap()),
            _ => unreachable!(),
        }
    }
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
        #[br(calc = DamageType::from_repr(param1 & 0x0F).unwrap())]
        #[bw(ignore)]
        damage_type: DamageType,
        #[br(calc = DamageElement::from_repr(param1 >> 4).unwrap())]
        #[bw(ignore)]
        damage_element: DamageElement,
        bonus_percent: u8,
        unk3: u8,
        unk4: u8,
        amount: u16,
    },
    #[brw(magic = 27u8)]
    BeginCombo,
    /// Seen during sprint.
    #[brw(magic = 14u8)]
    GainEffect {
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
    /// Seen during mounting.
    #[brw(magic = 39u8)]
    Mount { unk1: u8, unk2: u32, id: u16 },
    /// Seen during the Unveil action.
    #[brw(magic = 16u8)]
    LoseEffect {
        param: u16,
        unk: [u8; 3], // empty?
        effect_id: u16,
    },
}

#[repr(u8)]
#[derive(
    Debug, Eq, PartialEq, Clone, Copy, Default, Display, Deserialize, Serialize, FromRepr, EnumIter,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
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

impl mlua::IntoLua for DamageType {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

impl mlua::FromLua for DamageType {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Integer(integer) => Ok(Self::from_repr(integer as u8).unwrap()),
            _ => unreachable!(),
        }
    }
}

#[repr(u8)]
#[derive(
    Debug, Eq, PartialEq, Clone, Copy, Default, Display, Deserialize, Serialize, FromRepr, EnumIter,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
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

impl mlua::IntoLua for DamageElement {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

impl mlua::FromLua for DamageElement {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Integer(integer) => Ok(Self::from_repr(integer as u8).unwrap()),
            _ => unreachable!(),
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

    use crate::common::{INVALID_OBJECT_ID, ObjectId};

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_actionresult() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("action_result.bin"));

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
        d.push(server_zone_tests_dir!("action_result_sprint.bin"));

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
            EffectKind::GainEffect {
                unk1: 0,
                unk2: 7728,
                effect_id: 50,
                duration: 0.0,
                param: 0,
                source_actor_id: INVALID_OBJECT_ID
            }
        );

        assert_eq!(action_result.target_id_again.object_id, ObjectId(277554542));
    }

    #[test]
    fn read_actionresult_mount() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("action_result_mount.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_result = ActionResult::read_le(&mut buffer).unwrap();
        assert_eq!(action_result.main_target.object_id, ObjectId(277114100));
        assert_eq!(action_result.action_id, 55);
        assert_eq!(action_result.unk1, 4232092); // TODO: probably means this field is wrong
        assert_eq!(action_result.animation_lock_time, 0.1);
        assert_eq!(action_result.unk2, 3758096384); // TODO: ditto
        assert_eq!(action_result.hidden_animation, 4);
        assert_eq!(action_result.rotation, -0.8154669);
        assert_eq!(action_result.action_animation_id, 4);
        assert_eq!(action_result.variation, 0);
        assert_eq!(action_result.flag, 13);
        assert_eq!(action_result.unk3, 0);
        assert_eq!(action_result.effect_count, 1);
        assert_eq!(action_result.unk4, 0);
        assert_eq!(action_result.unk5, [0; 6]);

        // effect 0: unk2
        assert_eq!(
            action_result.effects[0].kind,
            EffectKind::Mount {
                unk1: 1,
                unk2: 0,
                id: 55,
            }
        );

        assert_eq!(action_result.target_id_again.object_id, ObjectId(277114100));
    }

    #[test]
    fn read_actionresult_unveil() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("action_result_unveil.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_result = ActionResult::read_le(&mut buffer).unwrap();
        assert_eq!(action_result.main_target.object_id, ObjectId(277114100));
        assert_eq!(action_result.action_id, 13266);
        assert_eq!(action_result.unk1, 749); // TODO: probably means this field is wrong
        assert_eq!(action_result.animation_lock_time, 0.6);
        assert_eq!(action_result.unk2, 3758096384); // TODO: ditto
        assert_eq!(action_result.hidden_animation, 18);
        assert_eq!(action_result.rotation, -2.0225368);
        assert_eq!(action_result.action_animation_id, 13266);
        assert_eq!(action_result.variation, 0);
        assert_eq!(action_result.flag, 1);
        assert_eq!(action_result.unk3, 0);
        assert_eq!(action_result.effect_count, 1);
        assert_eq!(action_result.unk4, 0);
        assert_eq!(action_result.unk5, [0; 6]);

        // effect 0: unk2
        assert_eq!(
            action_result.effects[0].kind,
            EffectKind::LoseEffect {
                param: 219,
                unk: [0; 3],
                effect_id: 565
            }
        );

        assert_eq!(action_result.target_id_again.object_id, ObjectId(277114100));
    }
}
