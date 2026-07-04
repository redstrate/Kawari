use diesel::{
    backend::Backend,
    deserialize::{self, FromSqlRow},
    expression::AsExpression,
    serialize,
    sql_types::Text,
    sqlite::Sqlite,
};
use kawari::ipc::zone::{
    GlamourDresserContents, GlamourPlate, GlamourPlateSaveAck, GlamourPlates,
};
use serde::{Deserialize, Serialize};

/// The number of glamour plates a character has.
pub const NUM_GLAMOUR_PLATES: usize = 20;
/// The number of slots per glamour plate.
pub const NUM_PLATE_SLOTS: usize = 12;

/// A single glamour plate as persisted to the database.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StoredPlate {
    /// The item ids for each slot. HQ items are encoded as `id | 0xF0000`.
    pub item_ids: [u32; NUM_PLATE_SLOTS],
    /// The first dye/stain for each slot.
    pub stain0: [u8; NUM_PLATE_SLOTS],
    /// The second dye/stain for each slot.
    pub stain1: [u8; NUM_PLATE_SLOTS],
}

impl Default for StoredPlate {
    fn default() -> Self {
        Self {
            item_ids: [0; NUM_PLATE_SLOTS],
            stain0: [0; NUM_PLATE_SLOTS],
            stain1: [0; NUM_PLATE_SLOTS],
        }
    }
}

impl From<&StoredPlate> for GlamourPlate {
    fn from(plate: &StoredPlate) -> Self {
        Self {
            item_ids: plate.item_ids,
            stain0: plate.stain0,
            stain1: plate.stain1,
        }
    }
}

/// A single item stored in the glamour dresser (prism box).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct StoredMirageItem {
    /// The item id. HQ items are encoded as `id | 0xF0000`, collectibles as `id | 0x100000`.
    pub item_id: u32,
    /// The item's two dyes/stains (stain0, stain1).
    pub stains: [u8; 2],
}

/// Persistent glamour data: the 20 glamour plates and the glamour dresser contents.
#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct GlamourStorage {
    pub plates: [StoredPlate; NUM_GLAMOUR_PLATES],
    pub dresser: Vec<StoredMirageItem>,
}

impl Default for GlamourStorage {
    fn default() -> Self {
        Self {
            plates: [StoredPlate::default(); NUM_GLAMOUR_PLATES],
            dresser: Vec::new(),
        }
    }
}

impl GlamourStorage {
    /// Builds the wire packet for a single dresser page. Items beyond what is stored are
    /// zero-filled.
    pub fn to_dresser_page(&self, page_index: u32) -> GlamourDresserContents {
        let mut page = GlamourDresserContents::empty_page(page_index);

        let start = page_index as usize * GlamourDresserContents::PAGE_SIZE;
        for slot in 0..GlamourDresserContents::PAGE_SIZE {
            if let Some(item) = self.dresser.get(start + slot) {
                page.item_ids[slot] = item.item_id;
                page.stain0[slot] = item.stains[0];
                page.stain1[slot] = item.stains[1];
            }
        }

        page
    }

    /// Stores an item in the first free dresser slot (or appends). Returns the index used.
    pub fn store_item(&mut self, item_id: u32, stains: [u8; 2]) -> usize {
        let entry = StoredMirageItem { item_id, stains };
        if let Some(index) = self.dresser.iter().position(|item| item.item_id == 0) {
            self.dresser[index] = entry;
            index
        } else {
            self.dresser.push(entry);
            self.dresser.len() - 1
        }
    }

    /// Removes the dresser entry at the given wire index (the slot the client
    /// addresses). Returns the removed entry, or None if the index is out of range
    /// or the slot is already empty. The slot is zeroed in place (rather than
    /// removed) to keep page indices stable.
    pub fn remove_by_index(&mut self, index: usize) -> Option<StoredMirageItem> {
        let entry = self.dresser.get_mut(index)?;
        if entry.item_id == 0 {
            return None;
        }
        let removed = *entry;
        *entry = StoredMirageItem::default();
        Some(removed)
    }

    /// Builds the wire packet for all 20 glamour plates.
    pub fn to_wire_plates(&self) -> GlamourPlates {
        GlamourPlates {
            plates: self.plates.iter().map(GlamourPlate::from).collect(),
        }
    }

    /// Applies a "save glamour plate" request using the per-slot `flags` from the 823 packet.
    ///
    /// Flag bits: bit0=has item (resolve glam_id), bit1=stain changed, bit2/0x04=empty slot.
    /// Wire stain value 0xFF means "no stain" and is stored as 0.
    /// Slots with flags==0 are left completely unchanged (item and stain preserved).
    pub fn save_plate(
        &mut self,
        plate_index: usize,
        flags: &[u8; NUM_PLATE_SLOTS],
        glam_indices: &[u32; NUM_PLATE_SLOTS],
        stain0: &[u8; NUM_PLATE_SLOTS],
        stain1: &[u8; NUM_PLATE_SLOTS],
    ) {
        if plate_index >= NUM_GLAMOUR_PLATES {
            return;
        }
        let plate = &mut self.plates[plate_index];
        for slot in 0..NUM_PLATE_SLOTS {
            let f = flags[slot];
            if f & 0x04 != 0 {
                // Explicit empty: clear item and stains.
                plate.item_ids[slot] = 0;
                plate.stain0[slot] = 0;
                plate.stain1[slot] = 0;
            } else {
                if f & 0x01 != 0 {
                    // Item changed: resolve dresser index to item_id snapshot.
                    let idx = glam_indices[slot] as usize;
                    if let Some(dresser_item) = self.dresser.get(idx) {
                        plate.item_ids[slot] = dresser_item.item_id;
                        // If the player did NOT explicitly re-dye this slot (bit1 absent),
                        // inherit the dresser item's own stains into the plate snapshot.
                        // The dresser item itself is never modified.
                        if f & 0x02 == 0 {
                            plate.stain0[slot] = dresser_item.stains[0];
                            plate.stain1[slot] = dresser_item.stains[1];
                        }
                    } else {
                        plate.item_ids[slot] = 0;
                    }
                }
                if f & 0x02 != 0 {
                    // Stain explicitly changed: use new value from packet (0xFF → 0 = no stain).
                    plate.stain0[slot] = if stain0[slot] == 0xFF { 0 } else { stain0[slot] };
                    plate.stain1[slot] = if stain1[slot] == 0xFF { 0 } else { stain1[slot] };
                }
                // flags == 0: no change; leave item_id and stains untouched.
            }
        }
    }

    /// Builds the 617 save-ack for a plate.
    /// Wire layout: plate_index + item_ids[12] + stain0[12] + stain1[12] + pad4.
    pub fn to_save_ack(&self, plate_index: usize) -> GlamourPlateSaveAck {
        let plate = &self.plates[plate_index];
        GlamourPlateSaveAck {
            plate_index: plate_index as u32,
            item_ids: plate.item_ids,
            stain0: plate.stain0,
            stain1: plate.stain1,
        }
    }
}

impl serialize::ToSql<Text, Sqlite> for GlamourStorage {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(serde_json::to_string(&self).unwrap());
        Ok(serialize::IsNull::No)
    }
}

impl deserialize::FromSql<Text, Sqlite> for GlamourStorage {
    fn from_sql(mut bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        Ok(serde_json::from_str(bytes.read_text())
            .ok()
            .unwrap_or_default())
    }
}
