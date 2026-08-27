use binrw::binrw;

use crate::common::{
    Position, read_packed_position, read_quantized_rotation, write_packed_position,
    write_quantized_rotation,
};

#[binrw]
#[repr(u8)]
#[brw(repr = u8)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WalkInEventType {
    /// Instantly teleport to the destination poisition.
    SkipAnimation = 0,
    /// Plays the animation and goes through all the control points. Used for most WalkInEvents.
    #[default]
    PlayAnimation = 1,
    /// Not sure what this is used for, plays at half speed and uses only half of the control points?
    Unk2 = 2,
}

#[binrw]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WalkInEvent {
    /// Object ID of the ClientPath in the zone.
    pub path_id: u32,
    /// Passed to the WalkInTriggerFinished ClientTrigger.
    pub unk1: u8,
    pub unk2: u8,
    pub unk3: u8,
    pub unk4: u8,
    /// The destination rotation.
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// Cast to a float, and divided by the 10 in the client.
    pub speed: u16,
    /// How the event should be played.
    pub event_type: WalkInEventType,
    pub unk6: u8,
    // The destination position.
    #[brw(pad_after = 4)] // unused
    #[br(map = read_packed_position)]
    #[bw(map = write_packed_position)]
    pub position: Position,
}
