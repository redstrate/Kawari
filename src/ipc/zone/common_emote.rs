use crate::common::{ObjectId, read_bool_from, write_bool_as};
use binrw::binrw;

/// The common structure used by both ActorControlTarget and ClientTrigger.
#[binrw]
#[brw(little)]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct CommonEmoteInfo {
    /// The id of the emote.
    emote: u32,
    /// 0/false = text shown, 1/true = text hidden
    #[brw(pad_before = 4)] // blank
    #[br(map = read_bool_from::<u32>)]
    #[bw(map = write_bool_as::<u32>)]
    hide_text: bool,
    /// The actor id of the target.
    #[brw(pad_before = 8)] // blank
    target: ObjectId,
    /// See the EmoteTargetType enum for more info.
    target_type: EmoteTargetType,
}

/// Information passed along to clients to let them know what kind of actor the emote is targeting.
#[binrw]
#[brw(repr = u32)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EmoteTargetType {
    /// None means there is no target, or it's a player or bnpc (monster, chocobo, carbuncle, scholar fairy).
    #[default]
    None = 0,
    /// Orchestrions, static NPCs in towns, etc.
    EObjOrNpc = 1,
    /// Player-summoned minions (not to be confused with chocobos or other bnpc pets).
    Minion = 4,
}
