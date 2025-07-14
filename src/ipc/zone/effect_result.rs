use binrw::binrw;

use crate::common::ObjectId;

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default)]
pub struct EffectEntry {
    pub index: u8,
    pub unk1: u8,
    pub id: u16,
    pub param: u16,
    pub unk2: u16,
    pub duration: f32,
    pub source_actor_id: ObjectId,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default)]
pub struct EffectResult {
    pub unk1: u32,
    pub unk2: u32,
    pub target_id: ObjectId,
    pub current_hp: u32,
    pub max_hp: u32,
    pub current_mp: u16,
    pub unk3: u8,
    pub class_id: u8,
    pub shield: u8,
    pub entry_count: u8,
    pub unk4: u16,
    #[brw(pad_after = 4)] // padding
    pub statuses: [EffectEntry; 4],
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use super::*;

    #[test]
    fn read_effectresult() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/effect_result.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let effect_result = EffectResult::read_le(&mut buffer).unwrap();
        assert_eq!(effect_result.unk1, 1);
        assert_eq!(effect_result.unk2, 776386);
        assert_eq!(effect_result.target_id, ObjectId(277554542));
    }
}
