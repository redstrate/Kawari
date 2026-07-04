use binrw::binrw;

/// A single glamour plate. There are 20 plates per character.
///
/// Slot order (12 slots): 0 mainhand, 1 offhand, 2 head, 3 body, 4 hands, 5 legs,
/// 6 feet, 7 ears, 8 neck, 9 wrist, 10 ring (right), 11 ring (left).
#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct GlamourPlate {
    /// The item ids for each slot. HQ items are encoded as `id | 0xF0000`.
    pub item_ids: [u32; 12],
    /// The first dye/stain for each slot.
    pub stain0: [u8; 12],
    /// The second dye/stain for each slot.
    pub stain1: [u8; 12],
}

impl GlamourPlate {
    pub const SIZE: usize = 72;
    pub const COUNT: usize = 20;
}

/// All 20 glamour plates for the character. Laid out contiguously with no outer header.
#[binrw]
#[derive(Debug, Clone)]
pub struct GlamourPlates {
    #[br(count = GlamourPlate::COUNT)]
    #[bw(pad_size_to = GlamourPlate::COUNT * GlamourPlate::SIZE)]
    pub plates: Vec<GlamourPlate>,
}

impl Default for GlamourPlates {
    fn default() -> Self {
        Self {
            plates: vec![GlamourPlate::default(); GlamourPlate::COUNT],
        }
    }
}

/// One page of glamour dresser (prism box) contents. The retail dresser holds 800
/// slots split across 2 pages of 400 items each (page 0 and page 1).
#[binrw]
#[derive(Debug, Clone)]
pub struct GlamourDresserContents {
    /// The index of this page, starting at 0.
    pub page_index: u32,
    /// The item ids stored in this page. HQ items are encoded as `id | 0xF0000`,
    /// collectibles as `id | 0x100000`.
    #[br(count = GlamourDresserContents::PAGE_SIZE)]
    #[bw(pad_size_to = GlamourDresserContents::PAGE_SIZE * 4)]
    pub item_ids: Vec<u32>,
    /// The per-slot first dye/stain, index-aligned with `item_ids`.
    #[br(count = GlamourDresserContents::PAGE_SIZE)]
    #[bw(pad_size_to = GlamourDresserContents::PAGE_SIZE)]
    pub stain0: Vec<u8>,
    /// The per-slot second dye/stain, index-aligned with `item_ids`.
    #[br(count = GlamourDresserContents::PAGE_SIZE)]
    #[bw(pad_size_to = GlamourDresserContents::PAGE_SIZE)]
    #[brw(pad_after = 4)] // reserved/padding, always zero in captures
    pub stain1: Vec<u8>,
}

impl GlamourDresserContents {
    /// The number of items in a single page.
    pub const PAGE_SIZE: usize = 400;
    /// The number of pages the retail dresser is split into.
    pub const PAGE_COUNT: usize = 2;

    /// Creates an empty (zero-filled) page with the given index.
    pub fn empty_page(page_index: u32) -> Self {
        Self {
            page_index,
            item_ids: vec![0; Self::PAGE_SIZE],
            stain0: vec![0; Self::PAGE_SIZE],
            stain1: vec![0; Self::PAGE_SIZE],
        }
    }
}

impl Default for GlamourDresserContents {
    fn default() -> Self {
        Self::empty_page(0)
    }
}

/// Acknowledges a glamour plate save.
///
/// Wire layout (80 bytes):
///   0x00  plate_index  u32
///   0x04  item_ids[12] u32×12  (slot order: 0=mainhand, 1=offhand, 2=head … 11=ring left)
///   0x34  stain0[12]   u8×12
///   0x40  stain1[12]   u8×12
///   0x4C  padding      4B
#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct GlamourPlateSaveAck {
    /// The plate that was saved (0..=19).
    pub plate_index: u32,
    /// Item ids for all 12 slots. Slot order: 0=mainhand, 1=offhand, 2=head, 3=body,
    /// 4=hands, 5=legs, 6=feet, 7=ears, 8=neck, 9=wrist, 10=ring(R), 11=ring(L).
    /// HQ items are encoded as `id | 0xF0000`.
    pub item_ids: [u32; 12],
    /// First dye/stain for each slot (same slot order as `item_ids`).
    pub stain0: [u8; 12],
    /// Second dye/stain for each slot (same slot order as `item_ids`).
    #[brw(pad_after = 4)]
    pub stain1: [u8; 12],
}

#[cfg(test)]
mod tests {
    use binrw::BinWrite;
    use std::io::Cursor;

    use super::*;

    #[test]
    fn glamour_plate_size() {
        crate::common::ensure_size::<GlamourPlate, { GlamourPlate::SIZE }>();
    }

    #[test]
    fn glamour_plates_size() {
        let mut cursor = Cursor::new(Vec::new());
        GlamourPlates::default().write_le(&mut cursor).unwrap();
        assert_eq!(
            cursor.position() as usize,
            GlamourPlate::SIZE * GlamourPlate::COUNT
        );
    }

    #[test]
    fn glamour_dresser_contents_size() {
        let mut cursor = Cursor::new(Vec::new());
        GlamourDresserContents::default()
            .write_le(&mut cursor)
            .unwrap();
        assert_eq!(cursor.position() as usize, 2408);
    }

    #[test]
    fn glamour_plate_save_ack_size() {
        let mut cursor = Cursor::new(Vec::new());
        GlamourPlateSaveAck::default().write_le(&mut cursor).unwrap();
        assert_eq!(cursor.position() as usize, 80);
    }
}
