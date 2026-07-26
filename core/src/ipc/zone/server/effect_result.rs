use binrw::binrw;

use crate::common::ObjectId;

#[binrw]
#[brw(little)]
#[derive(Clone, Copy, Debug, Default)]
pub struct EffectEntry {
    pub index: u8,
    pub unk1: u8,
    /// Index into the Status Excel sheet.
    pub id: u16,
    /// Status-specific parameter.
    pub param: u16,
    pub unk2: u16,
    /// How long the effect should last for.
    pub duration: f32,
    pub source_actor_id: ObjectId,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default)]
pub struct EffectResult {
    /// How many actors are covered by this packet.
    pub count: u32,
    /// See ActionResult for more details.
    pub global_sequence: u32,
    pub target_id: ObjectId,
    /// Amount of health points.
    pub health_points: u32,
    /// Maximum amount of health points.
    pub max_health_points: u32,
    /// Amount of resource points (MP/GP/CP etc.)
    pub resource_points: u16,
    pub target_index: u8,
    pub classjob_id: u8,
    pub shield: u8,
    pub entry_count: u8,
    #[brw(pad_before = 2)] // not read
    #[brw(pad_after = 4)] // padding
    pub statuses: [EffectEntry; 4],
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_effectresult() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("effect_result.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let effect_result = EffectResult::read_le(&mut buffer).unwrap();
        assert_eq!(effect_result.count, 1);
        assert_eq!(effect_result.global_sequence, 776386);
        assert_eq!(effect_result.target_id, ObjectId(277554542));
    }
}
