use binrw::binrw;

/// Sent by the client when saving a glamour plate from the glamour dresser (prism box).
///
/// The 12-slot order is: 0 mainhand, 1 offhand, 2 head, 3 body, 4 hands, 5 legs, 6 feet,
/// 7 ears, 8 neck, 9 wrist, 10 ring (right), 11 ring (left).
///
/// The fields the server actually consumes are `plate_index`, `glam_indices`, `stain0` and
/// `stain1`. The remaining fields are read so packet framing stays correct (the body must be
/// exactly 248 bytes), but are not otherwise used.
#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct SaveGlamourPlate {
    /// The plate being saved (0..=19).
    pub plate_index: u8,
    /// Per-slot dye source flags.
    pub stain_source: [u8; 12],
    /// Per-slot flags.
    pub flags: [u8; 12],
    /// 3 uninitialized gap bytes before the glamour indices.
    #[br(pad_before = 3)]
    #[bw(pad_before = 3)]
    /// Dresser indices for each slot. 0 means "empty slot".
    pub glam_indices: [u32; 12],
    /// Per-slot first dye container.
    pub dye0_container: [u32; 12],
    /// Per-slot first dye index.
    pub dye0_index: [u16; 12],
    /// Per-slot second dye container.
    pub dye1_container: [u32; 12],
    /// Per-slot second dye index.
    pub dye1_index: [u16; 12],
    /// The first dye/stain for each slot.
    pub stain0: [u8; 12],
    /// The second dye/stain for each slot.
    #[brw(pad_after = 4)]
    pub stain1: [u8; 12],
}

#[cfg(test)]
mod tests {
    use binrw::BinWrite;
    use std::io::Cursor;

    use super::*;

    #[test]
    fn save_glamour_plate_size() {
        let mut cursor = Cursor::new(Vec::new());
        SaveGlamourPlate::default().write_le(&mut cursor).unwrap();
        assert_eq!(cursor.position() as usize, 248);
    }
}
