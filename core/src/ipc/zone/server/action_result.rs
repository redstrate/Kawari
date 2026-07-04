use binrw::binrw;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, FromRepr};

use crate::{
    common::{ObjectId, ObjectTypeId, read_quantized_rotation, write_quantized_rotation},
    ipc::zone::ActionType,
};

const DAMAGE_HEAL_LARGE_VALUE_FLAG: u8 = 0x40;
const DAMAGE_HEAL_MAX_VALUE: u32 = 0x00FF_FFFF;

fn decode_damage_heal_amount(value: u16, param3: u8, param4: u8) -> u32 {
    let amount = u32::from(value);
    if param4 & DAMAGE_HEAL_LARGE_VALUE_FLAG != 0 {
        amount + (u32::from(param3) << 16)
    } else {
        amount
    }
}

fn encode_damage_heal_amount_low(amount: u32) -> u16 {
    (amount.min(DAMAGE_HEAL_MAX_VALUE) & 0xFFFF) as u16
}

fn encode_damage_heal_amount_high(amount: u32) -> u8 {
    ((amount.min(DAMAGE_HEAL_MAX_VALUE) >> 16) & 0xFF) as u8
}

fn encode_damage_heal_param4(amount: u32, param4: u8) -> u8 {
    let mut param4 = param4 & !DAMAGE_HEAL_LARGE_VALUE_FLAG;
    if amount.min(DAMAGE_HEAL_MAX_VALUE) > u32::from(u16::MAX) {
        param4 |= DAMAGE_HEAL_LARGE_VALUE_FLAG;
    }
    param4
}

fn encode_heal_params(amount: u32, params: [u8; 5]) -> [u8; 5] {
    [
        params[0],
        params[1],
        params[2],
        encode_damage_heal_amount_high(amount),
        encode_damage_heal_param4(amount, params[4]),
    ]
}

// TODO: this might be a flag?
#[binrw]
#[derive(
    Debug, Eq, PartialEq, Clone, Copy, Default, Display, Deserialize, Serialize, EnumIter, FromRepr,
)]
#[repr(u8)]
#[brw(repr = u8)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum DamageKind {
    // These are all "normal" attacks, not direct hits/criticals or anything special.
    #[default]
    Normal = 0x0,
    Critical = 0x1,
    DirectHit = 0x2,
    /// Both a critical *and* a direct hit (the severity field is a bitfield: 0x1 | 0x2).
    CriticalDirectHit = 0x3,
}

#[cfg(feature = "server")]
impl mlua::IntoLua for DamageKind {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

#[cfg(feature = "server")]
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
    /// There's no effect entry.
    #[brw(magic = 0u8)]
    None,
    /// The attack missed.
    #[brw(magic = 1u8)]
    Miss,
    /// Do damage!
    #[brw(magic = 3u8)]
    Damage {
        damage_kind: DamageKind,
        #[br(temp)]
        #[bw(ignore)]
        param1: u8,
        #[br(calc = DamageType::from_repr(param1 & 0x0F).unwrap())]
        #[bw(ignore)]
        damage_type: DamageType,
        #[br(calc = DamageElement::from_repr(param1 >> 4).unwrap())]
        #[bw(ignore)]
        damage_element: DamageElement,
        #[br(ignore)]
        #[bw(calc = ((*damage_element as u8) << 4) | *damage_type as u8)]
        actual_param1: u8,
        bonus_percent: u8,
        #[br(temp)]
        #[bw(calc = encode_damage_heal_amount_high(*amount))]
        param3: u8,
        #[br(temp)]
        #[bw(calc = encode_damage_heal_param4(*amount, *unk4))]
        param4: u8,
        #[br(temp)]
        #[bw(calc = encode_damage_heal_amount_low(*amount))]
        value: u16,
        #[br(calc = param3)]
        #[bw(ignore)]
        unk3: u8,
        #[br(calc = param4)]
        #[bw(ignore)]
        unk4: u8,
        #[br(calc = decode_damage_heal_amount(value, param3, param4))]
        #[bw(ignore)]
        amount: u32,
    },
    /// Heals for a specified amount.
    #[brw(magic = 4u8)]
    Heal {
        #[br(temp)]
        #[bw(calc = encode_heal_params(*amount, *unk1))]
        params: [u8; 5],
        #[br(temp)]
        #[bw(calc = encode_damage_heal_amount_low(*amount))]
        value: u16,
        #[br(calc = params)]
        #[bw(ignore)]
        unk1: [u8; 5],
        #[br(calc = decode_damage_heal_amount(value, params[3], params[4]))]
        #[bw(ignore)]
        amount: u32,
    },
    /// Seen while attacking giant clams.
    #[brw(magic = 7u8)]
    Invincible {},
    /// Seen during Head Graze.
    #[brw(magic = 8u8)]
    InterruptAction {},
    /// Executes/combies an action combo.
    #[brw(magic = 27u8)]
    ExecuteCombo {
        /// Unknown, but seen set to 1 during Fountain (which comboes with Cascade.)
        sequence: u8,
        unk2: u8,
        unk3: u8,
        unk4: u8,
        unk5: u8,
        /// Which action begun this combo, I guess.
        action_id: u16,
    },
    /// Seen during Sprint.
    #[brw(magic = 14u8)]
    GainEffect {
        unk1: u8,
        unk2: u8,
        /// Status-specific parameter.
        param: u16,
        unk3: u8,
        /// Index into the Status Excel sheet.
        effect_id: u16,

        // NOTE: the following is for our internal usage, this is not an actual part of the packet
        // TODO: this shouldn't be here, instead we should maybe create a lua-specific struct for all of this information
        #[brw(ignore)]
        duration: f32,
    },
    /// Seen on Summon Bahamut / Summon Solar Bahamut. The payload is `00 00 00 00 00 01 00` on
    /// retail and appears to be the demi-summon transition effect distinct from ordinary pet summon.
    #[brw(magic = 25u8)]
    SummonDemi { unk: [u8; 7] },
    /// Seen during Cascade (and gaining Silken Symmetry.)
    /// Guessed at it's purpose, not 100% certain it's for applying to yourself.
    #[brw(magic = 15u8)]
    GainEffectSelf {
        unk1: u8,
        unk2: u8,
        /// Status-specific parameter.
        param: u16,
        unk3: u8,
        /// Index into the Status Excel sheet.
        effect_id: u16,

        // NOTE: the following is for our internal usage, this is not an actual part of the packet
        // TODO: this shouldn't be here, instead we should maybe create a lua-specific struct for all of this information
        #[brw(ignore)]
        duration: f32,
    },
    /// Seen during the Unveil action.
    #[brw(magic = 16u8)]
    LoseEffect {
        param: u16,
        unk: [u8; 3], // empty?
        effect_id: u16,
    },
    /// Seen during mounting.
    #[brw(magic = 39u8)]
    Mount { unk1: u8, unk2: u32, id: u16 },
    /// Play this VFX.
    #[brw(magic = 59u8)]
    PlayVFX { unk: [u8; 5], effect_id: u16 },
    /// Seen in the Summon Carbuncle action.
    #[brw(magic = 62u8)]
    SummonPet { unk: [u8; 7] },
    /// Unknown effect (that should be added!)
    Unknown { magic: u8, unk: [u8; 7] },
}

#[repr(u8)]
#[derive(
    Debug, Eq, PartialEq, Clone, Copy, Default, Display, Deserialize, Serialize, FromRepr, EnumIter,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum DamageType {
    /// Usually reserved for special enemy actions.
    Unique,
    Slashing,
    Piercing,
    Blunt,
    Shot,
    /// Magical damage makes up the breadth of most player spells.
    Magic,
    Breath,
    /// Physical damage makes up the breadth of most player weaponskills.
    #[default]
    Physical,
    LimitBreak,
}

#[cfg(feature = "server")]
impl mlua::IntoLua for DamageType {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
impl mlua::IntoLua for DamageElement {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

#[cfg(feature = "server")]
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
#[derive(Clone, Copy, Eq, PartialEq, Default)]
pub struct ActionResultFlag(u8);

bitflags! {
    impl ActionResultFlag : u8 {
        const FORCE_ANIMATION_LOCK = 0x1;
    }
}

impl std::fmt::Debug for ActionResultFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct ActionResult {
    pub animation_target_id: ObjectTypeId,
    /// Index into the Action Excel sheet.
    pub action_id: u32,
    pub global_sequence: u32,
    /// Controls how long the next action should be delayed, in seconds.
    pub animation_lock: f32,
    /// Only used when ActionCategory is 11.
    pub ballista_entity_id: ObjectId,
    /// The same as `sequence` from this action's `ActionRequest`.
    pub source_sequence: u16,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// Usually the same as `action_id`.
    pub spell_id: u16,
    pub animation_variation: u8,
    /// The kind of action.
    pub action_type: ActionType,
    pub flags: ActionResultFlag,
    pub effect_count: u8,
    pub unk4: u16,
    pub unk5: [u8; 6], // might be not read by the client?
    pub effects: [ActionEffect; 8],
    #[brw(pad_before = 6, pad_after = 4)]
    pub target_id_again: ObjectTypeId,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::{BinRead, BinWrite};

    use crate::common::ObjectId;

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn action_effect_damage_uses_large_value_encoding() {
        let effect = ActionEffect {
            kind: EffectKind::Damage {
                damage_kind: DamageKind::Normal,
                damage_type: DamageType::Magic,
                damage_element: DamageElement::Unaspected,
                bonus_percent: 0,
                unk3: 0,
                unk4: 0,
                amount: 70_000,
            },
        };

        let mut writer = Cursor::new(Vec::new());
        effect.write_le(&mut writer).unwrap();
        let raw = writer.into_inner();

        assert_eq!(raw.len(), 8);
        assert_eq!(raw[0], 3);
        assert_eq!(raw[4], 1);
        assert_eq!(raw[5] & 0x40, 0x40);
        assert_eq!(u16::from_le_bytes([raw[6], raw[7]]), 4_464);

        let mut reader = Cursor::new(raw);
        let parsed = ActionEffect::read_le(&mut reader).unwrap();
        assert_eq!(
            parsed.kind,
            EffectKind::Damage {
                damage_kind: DamageKind::Normal,
                damage_type: DamageType::Magic,
                damage_element: DamageElement::Unaspected,
                bonus_percent: 0,
                unk3: 1,
                unk4: 0x40,
                amount: 70_000,
            }
        );
    }

    #[test]
    fn read_actionresult() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("action_result.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_result = ActionResult::read_le(&mut buffer).unwrap();
        assert_eq!(
            action_result.animation_target_id.object_id,
            ObjectId(0x40070E42)
        );
        assert_eq!(action_result.action_id, 31);
        assert_eq!(action_result.global_sequence, 2662353);
        assert_eq!(action_result.animation_lock, 0.6);
        assert_eq!(action_result.ballista_entity_id, ObjectId::default());
        assert_eq!(action_result.source_sequence, 1);
        assert_eq!(action_result.rotation, 1.207309);
        assert_eq!(action_result.spell_id, 31);
        assert_eq!(action_result.animation_variation, 0);
        assert_eq!(action_result.flags, ActionResultFlag::empty());
        assert_eq!(action_result.action_type, ActionType::Action);
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
        assert_eq!(
            action_result.effects[1].kind,
            EffectKind::ExecuteCombo {
                sequence: 0,
                unk2: 0,
                unk3: 0,
                unk4: 0,
                unk5: 128,
                action_id: 31
            }
        );

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
        assert_eq!(
            action_result.animation_target_id.object_id,
            ObjectId(277554542)
        );
        assert_eq!(action_result.action_id, 3);
        assert_eq!(action_result.global_sequence, 776386);
        assert_eq!(action_result.animation_lock, 0.6);
        assert_eq!(action_result.ballista_entity_id, ObjectId::default());
        assert_eq!(action_result.source_sequence, 1);
        assert_eq!(action_result.rotation, 2.6254003);
        assert_eq!(action_result.spell_id, 3);
        assert_eq!(action_result.animation_variation, 0);
        assert_eq!(action_result.flags, ActionResultFlag::empty());
        assert_eq!(action_result.action_type, ActionType::Action);
        assert_eq!(action_result.effect_count, 1);
        assert_eq!(action_result.unk4, 0);
        assert_eq!(action_result.unk5, [0; 6]);

        assert_eq!(
            action_result.effects[0].kind,
            EffectKind::GainEffect {
                unk1: 0,
                unk2: 48,
                unk3: 0,
                effect_id: 50,
                duration: 0.0,
                param: 30,
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
        assert_eq!(
            action_result.animation_target_id.object_id,
            ObjectId(277114100)
        );
        assert_eq!(action_result.action_id, 55);
        assert_eq!(action_result.global_sequence, 4232092);
        assert_eq!(action_result.animation_lock, 0.1);
        assert_eq!(action_result.ballista_entity_id, ObjectId::default());
        assert_eq!(action_result.source_sequence, 4);
        assert_eq!(action_result.rotation, -0.8154669);
        assert_eq!(action_result.spell_id, 4);
        assert_eq!(action_result.animation_variation, 0);
        assert_eq!(action_result.flags, ActionResultFlag::empty());
        assert_eq!(action_result.action_type, ActionType::Mount);
        assert_eq!(action_result.effect_count, 1);
        assert_eq!(action_result.unk4, 0);
        assert_eq!(action_result.unk5, [0; 6]);

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
        assert_eq!(
            action_result.animation_target_id.object_id,
            ObjectId(277114100)
        );
        assert_eq!(action_result.action_id, 13266);
        assert_eq!(action_result.global_sequence, 749);
        assert_eq!(action_result.animation_lock, 0.6);
        assert_eq!(action_result.ballista_entity_id, ObjectId::default());
        assert_eq!(action_result.source_sequence, 18);
        assert_eq!(action_result.rotation, -2.0225368);
        assert_eq!(action_result.spell_id, 13266);
        assert_eq!(action_result.animation_variation, 0);
        assert_eq!(action_result.flags, ActionResultFlag::empty());
        assert_eq!(action_result.action_type, ActionType::Action);
        assert_eq!(action_result.effect_count, 1);
        assert_eq!(action_result.unk4, 0);
        assert_eq!(action_result.unk5, [0; 6]);

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
