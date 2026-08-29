use crate::common::{CHAR_NAME_MAX_LENGTH, ObjectId, read_string, write_string};
use binrw::binrw;

#[binrw]
#[derive(Clone, Debug)]
pub struct ZoneDiceRollResult {
    /// The sender's account id.
    pub account_id: u64,
    /// The sender's content id.
    pub content_id: u64,
    /// The sender's actor id.
    pub actor_id: ObjectId,
    /// The sender's world id.
    pub world_id: u16,
    /// The result of the dice roll.
    pub roll_result: u16,
    /// The number of sides the die had.
    pub num_sides: u16,
    pub unk: u16, // Always 0x100?
    /// The sending character's name.
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 4)] // Empty/padding
    pub name: String,
}
