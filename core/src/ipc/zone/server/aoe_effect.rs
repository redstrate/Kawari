//! Multi-target action-effect packets (`AoeEffect8/16/24/32`).
//!
//! These mirror the client's `ActionEffectN` structs (see ffxiv_bossmod `ServerIPC.cs`). A single
//! packet carries up to N targets, each with its own 8-slot effect array (so e.g. damage numbers
//! show up on every target hit by one AoE). The server picks the smallest variant that holds all
//! affected targets; anything past 32 is dropped (its damage is silently swallowed, matching how
//! retail caps a single effect packet).
//!
//! Wire layout (little-endian, `Pack = 1`):
//! ```text
//! AoeEffectHeader (42 bytes)
//! effects:   [[ActionEffect; 8]; N]   (64 * N bytes)
//! padding3:  u16
//! padding4:  u32
//! target_ids:[ObjectTypeId; N]        (8 * N bytes)
//! target_x/y/z: u16 * 3               (packed position of the AoE center)
//! padding5:  u16
//! padding6:  u32
//! ```
//! Total size = 60 + 72 * N (636 / 1212 / 1788 / 2364 for N = 8 / 16 / 24 / 32).

use binrw::binrw;

use crate::common::{
    ObjectId, ObjectTypeId, Position, read_packed_position, read_quantized_rotation,
    write_packed_position, write_quantized_rotation,
};
use crate::ipc::zone::ActionType;

use super::action_result::{ActionEffect, ActionResultFlag};

/// The 42-byte header shared by every `AoeEffectN` packet. Field semantics match
/// `ActionResult`/the client's `ActionEffectHeader`.
#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct AoeEffectHeader {
    /// Who the animation targets (usually the primary/clicked target).
    pub animation_target_id: ObjectTypeId,
    /// Index into the Action Excel sheet.
    pub action_id: u32,
    pub global_sequence: u32,
    /// How long the next action is delayed, in seconds.
    pub animation_lock: f32,
    /// Only used when ActionCategory is 11.
    pub ballista_entity_id: ObjectId,
    /// The same as `sequence` from this action's `ActionRequest`.
    pub source_sequence: u16,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// Usually the same as `action_id`.
    pub spell_id: u16,
    pub animation_variation: u8,
    pub action_type: ActionType,
    pub flags: ActionResultFlag,
    /// Number of populated target slots.
    #[brw(pad_after = 8)] // padding21..24
    pub target_count: u8,
}

/// Generates an `AoeEffectN` struct with a fixed `N`-target capacity.
macro_rules! aoe_effect_struct {
    ($name:ident, $n:expr) => {
        #[binrw]
        #[brw(little)]
        #[derive(Debug, Clone)]
        pub struct $name {
            pub header: AoeEffectHeader,
            /// Per-target effect rows; each target gets its own 8-slot effect array.
            pub effects: [[ActionEffect; 8]; $n],
            // padding3 (u16) + padding4 (u32) — BOTH sit *before* target_ids (verified against a
            // native AoeEffect8: the first target id starts 6 bytes after the effects array, with
            // the position field immediately following the target_ids). Splitting this as
            // 2-before/4-after shifted every target id 4 bytes early, so the client read target 0's
            // id from the padding (= 0 / no-target) and showed the hit as no-damage.
            #[brw(pad_before = 6)]
            pub target_ids: [ObjectTypeId; $n],
            /// Packed position of the AoE center.
            #[brw(pad_after = 6)] // padding5 (u16) + padding6 (u32)
            #[br(map = read_packed_position)]
            #[bw(map = write_packed_position)]
            pub position: Position,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    header: AoeEffectHeader::default(),
                    effects: [[ActionEffect::default(); 8]; $n],
                    target_ids: [ObjectTypeId::default(); $n],
                    position: Position::default(),
                }
            }
        }
    };
}

aoe_effect_struct!(AoeEffect8, 8);
aoe_effect_struct!(AoeEffect16, 16);
aoe_effect_struct!(AoeEffect24, 24);
aoe_effect_struct!(AoeEffect32, 32);
