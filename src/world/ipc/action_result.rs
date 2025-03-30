use binrw::binrw;

use crate::common::{ObjectTypeId, read_quantized_rotation, write_quantized_rotation};

#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, Copy, Default)]
#[brw(repr = u8)]
pub enum EffectKind {
    #[default]
    Damage = 3,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionEffect {
    pub kind: EffectKind,
    pub param0: u8,
    pub param1: u8,
    pub param2: u8,
    pub param3: u8,
    pub param4: u8,
    pub value: u16,
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
        assert_eq!(action_result.main_target.object_id, ObjectId(0x400097d0));
        assert_eq!(
            action_result.target_id_again.object_id,
            ObjectId(0x400097d0)
        );
        assert_eq!(action_result.action_id, 31);
        assert_eq!(action_result.animation_lock_time, 0.6);
        assert_eq!(action_result.rotation, 1.9694216);
        assert_eq!(action_result.action_animation_id, 31);
        assert_eq!(action_result.flag, 1);
        assert_eq!(action_result.effect_count, 1);

        // effect 0: attack
        assert_eq!(action_result.effects[0].action_type, 3);
        assert_eq!(action_result.effects[0].param0, 0);
        assert_eq!(action_result.effects[0].param1, 113);
        assert_eq!(action_result.effects[0].param2, 0);
        assert_eq!(action_result.effects[0].param3, 0);
        assert_eq!(action_result.effects[0].param4, 0);
        assert_eq!(action_result.effects[0].value, 22);

        // effect 1: start action combo
        assert_eq!(action_result.effects[1].action_type, 27);
    }
}
