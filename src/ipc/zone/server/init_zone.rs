use binrw::binrw;
use bitflags::bitflags;

use crate::common::Position;

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InitZoneFlags(pub u16);

impl std::fmt::Debug for InitZoneFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

bitflags! {
    impl InitZoneFlags : u16 {
        /// No flags.
        const NONE = 0x000;

        /// Enables the Playguide window, and also the Duty Recorder. Only sent for the first zone logged into.
        const INITIAL_LOGIN = 0x001;

        // TODO: no idea, I didn't find this in the disassembly so it may be unused/no effect. Set while in an instanced duty (explorer mode.)
        const UNK1 = 0x002;

        // TODO: I think this is for resetting the content finder queue info? This is set when returning from a duty.
        const UNK2 = 0x004;

        /// Hides the server information in the status bar.
        const HIDE_SERVER = 0x008;

        /// Allows flying on your mount. This only works if you completed A Realm Reborn.
        const ENABLE_FLYING = 0x010;

        // TODO: 32 seems to control some UI state. Set while in an instanced duty (explorer mode.)
        const UNK3 = 0x020;

        /// Informs the client that this is an instanced area. Also needs instance_id to be a non-zero value.
        const INSTANCED_AREA = 0x080;

        // TODO: 256 seems to be something else UI related

        // TODO: 512 seems to be something weather-related?
    }
}

impl Default for InitZoneFlags {
    fn default() -> Self {
        Self::NONE
    }
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct InitZone {
    /// This is the internal server ID. (*Not* the World ID.) This seems to be just for informational purposes, and doesn't affect anything functionally. Always the same as the `server_id` in `IpcSegment`.
    pub server_id: u16,
    /// Index into the TerritoryType Excel sheet.
    pub territory_type: u16,
    /// The id of the instanced area, has no effect if non-zero and flags doesn't contain `INSTANCED_AREA`.
    pub instance_id: u16,
    pub content_finder_condition_id: u16,
    pub transition_territory_filter_key: u32,
    /// Unknown purpose.
    pub layout_id: u32,
    /// Index into the Weather Excel sheet.
    #[brw(pad_after = 1)]
    // Empty. Currently it's read as a byte, however it's more than likely going to change into a u16 in the future.
    pub weather_id: u8,
    pub flags: InitZoneFlags,
    pub unk_bitmask1: u8, // unknown purpose, seems to always be 170 for me. 168 in instanced areas.
    /// Zero means "no obsfucation" (not really, but functionally yes.)
    /// To enable obsfucation, you need to set this to a constant that changes every patch. See lib.rs for the constant.
    pub obsfucation_mode: u8,
    /// First seed used in deobsfucation on the client side.
    pub seed1: u8,
    /// Second seed used in deobsfucation on the client side.
    pub seed2: u8,
    /// Third seed used in deobsfucation on the client side.
    pub seed3: u32,
    pub unk7: [u8; 18],
    /// Might be the festivals active in the current zone? Unsure.
    pub festivals_id1: [u16; 4],
    pub festivals_phase1: [u16; 4],
    /// Might be festivals active on the current server? Unsure.
    pub festivals_id2: [u16; 4],
    pub festivals_phase2: [u16; 4],
    pub unk8_9: [u8; 2],
    /// This gives a hint to level streaming so it can preload this area.
    pub position: Position,
    pub unk8: [u32; 4],
    pub unk9: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_init_zone() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("init_zone.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let init_zone = InitZone::read_le(&mut buffer).unwrap();
        assert_eq!(init_zone.server_id, 1);
        assert_eq!(init_zone.territory_type, 182);
        assert_eq!(init_zone.instance_id, 0);
        assert_eq!(init_zone.weather_id, 2);
        assert_eq!(init_zone.position.x, 40.519722);
        assert_eq!(init_zone.position.y, 4.0);
        assert_eq!(init_zone.position.z, -150.33124);
    }
}
