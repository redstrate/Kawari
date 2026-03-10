use std::convert::{From, Into};
use std::fmt;

use binrw::binrw;

/// A type describing whether the waymark was placed or removed.
#[binrw]
#[repr(u8)]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum WaymarkPlacementMode {
    #[default]
    /// The waymark was removed.
    Removed = 0,
    /// The waymark was placed.
    Placed = 1,
}

/// A factor used for `WaymarkCoordinate`s in the conversion to and from f32 & i32.
const COORDINATE_FACTOR: f32 = 1000.0;

/// A special type specific to waymarks that converts 32-bit floats to 32-bit signed integers.
/// The game client sends i32s to the server and expects them back in this same format.
/// We also implement Debug and Display so that PacketAnalyzer can view them correctly.
#[binrw]
#[brw(little)]
#[derive(Clone, Copy, Default, PartialEq)]
pub struct WaymarkCoordinate(pub i32);

impl From<f32> for WaymarkCoordinate {
    fn from(value: f32) -> Self {
        Self((value * COORDINATE_FACTOR) as i32)
    }
}

impl fmt::Debug for WaymarkCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for WaymarkCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.3}", (self.0 as f32 / COORDINATE_FACTOR))
    }
}

/// A higher-level struct used to group the waymark's 3D coordinates easier.
#[binrw]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WaymarkPosition {
    /// The waymark's X coordinate.
    pub x: WaymarkCoordinate,
    /// The waymark's Y coordinate.
    pub y: WaymarkCoordinate,
    /// The waymark's Z coordinate.
    pub z: WaymarkCoordinate,
}

// The current amount of waymarkers available for the player's party to use.
const NUM_WAYMARKERS: usize = 8;

/// A type internal to Kawari which allows easier conversion to and from the game client's expected format.
pub type WaymarkPositions = [Option<WaymarkPosition>; NUM_WAYMARKERS];

/// The game's interpretation of a waymark preset. Used by both client and server.
/// We also provide some helpers below to display the data in a more sensible manner than just raw arrays, as well as convert to a simpler format for us to reuse in our party implementation.
#[binrw]
#[brw(little)]
#[derive(Clone, Copy, Default)]
pub struct WaymarkPreset {
    /// A bitmask describing which of the waymarks in this preset are enabled.
    #[brw(pad_after = 3)] // empty/zeroes
    pub enabled_bitmask: u8,
    /// All of the waymarks' X coordinates.
    pub x: [WaymarkCoordinate; NUM_WAYMARKERS],
    /// All of the waymarks' Y coordinates.
    pub y: [WaymarkCoordinate; NUM_WAYMARKERS],
    /// All of the waymarks' Z coordinates.
    #[brw(pad_after = 4)] // empty/zeroes
    pub z: [WaymarkCoordinate; NUM_WAYMARKERS],
}

impl fmt::Display for WaymarkPreset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let waymark_data: WaymarkPositions = (*self).into();
        write!(f, "{:#?}", waymark_data)
    }
}

impl fmt::Debug for WaymarkPreset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<WaymarkPositions> for WaymarkPreset {
    fn from(positions: WaymarkPositions) -> Self {
        let mut ret = WaymarkPreset::default();
        for (i, position_data) in positions.iter().enumerate() {
            if let Some(position_data) = position_data {
                ret.enabled_bitmask |= (1 << i) as u8;
                ret.x[i] = position_data.x;
                ret.y[i] = position_data.y;
                ret.z[i] = position_data.z;
            }
        }

        ret
    }
}

impl From<WaymarkPreset> for WaymarkPositions {
    fn from(preset: WaymarkPreset) -> WaymarkPositions {
        let mut waymarks = WaymarkPositions::default();
        for (i, position_data) in waymarks.iter_mut().enumerate().take(NUM_WAYMARKERS) {
            if preset.enabled_bitmask & (1 << i) != 0 {
                *position_data = Some(WaymarkPosition {
                    x: preset.x[i],
                    y: preset.y[i],
                    z: preset.z[i],
                });
            }
        }

        waymarks
    }
}
