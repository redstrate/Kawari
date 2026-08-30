use crate::{
    common::{CHAR_NAME_MAX_LENGTH, read_string, write_string},
    ipc::chat::ChatChannel,
};
use binrw::binrw;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct ChatDiceRollResult {
    /// The destination ChatChannel.
    pub community_id: ChatChannel,
    /// The account id of the sender.
    pub account_id: u64,
    /// The content id of the sender.
    pub content_id: u64,
    /// The world id of the sender.
    pub world_id: u16,
    /// The result of the dice roll.
    pub roll_result: u16,
    /// The number of sides on the die.
    pub num_sides: u16,
    pub unk1: u8, // Seems to echo ChatDiceRollData's unk2?
    pub unk2: u8, // Seems to echo ChatDiceRollData's unk1? Getting either of these unks wrong results in the client seeingly ignoring our response
    /// The sending character's name.
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_name: String,
}
