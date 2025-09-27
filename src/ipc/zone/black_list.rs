use binrw::binrw;

/// A type describing a character contained in the main blacklist.
#[binrw]
#[derive(Debug, Copy, Clone, Default)]
pub struct BlacklistedCharacter {
    /// The blocked character's content id, or account id, unclear which.
    pub content_id: u64,
    /// An unknown flag/boolean, possibly related to blocking chat messages, or visibility when in the same zone.
    pub flag1: u16, // Assumed, seems to be set to 1
    // TODO: The padding after seems to be empty, but more testing needs to be done to see if it doesn't hold a world id or other info for players that aren't from the client's world
    #[brw(pad_after = 4)]
    /// Another unknown flag/boolean, possibly related to blocking chat messages, or visibility when in the same zone.
    pub flag2: u16, // Assumed, seems to be set to 1
}

impl BlacklistedCharacter {
    pub const SIZE: usize = 16;
}

/// The main list the server responds with, containing a number of blocked characters/accounts per request.
#[binrw]
#[derive(Debug, Default, Clone)]
pub struct Blacklist {
    /// The actual blocked chara/account data.
    #[brw(pad_after = 1)]
    #[br(count = Blacklist::NUM_ENTRIES)]
    #[bw(pad_size_to = Blacklist::NUM_ENTRIES * BlacklistedCharacter::SIZE)]
    pub data: Vec<BlacklistedCharacter>, // TODO: How many actually fit in here? This matches the packet size, but it's unclear if it sends fewer
    /// A sequence value used for bookkeeping/synchronization. It matches the one sent by the client.
    #[brw(pad_after = 5)]
    pub sequence: u16,
}

impl Blacklist {
    pub const NUM_ENTRIES: usize = 60;
}

/// The request sent by the client, to obtain the list of blocked characters/accounts.
#[binrw]
#[derive(Debug, Default, Copy, Clone)]
pub struct RequestBlacklist {
    /// The sequence value sent by the client for this request.
    #[brw(pad_after = 6)] // TODO: Empty? It's unclear if sequence is a larger integer type
    pub sequence: u16,
}
