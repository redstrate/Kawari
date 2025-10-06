use binrw::binrw;
use bitflags::bitflags;

use crate::common::Position;

// NOTE: Not 100% sure this is actually u16, it could be u8.
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
        /// Hides the server information in the status bar.
        const HIDE_SERVER = 0x008;
        /// Allows flying on your mount. This only works if you completed A Realm Reborn.
        const ENABLE_FLYING = 0x010;
        /// Informs the client that this is an instanced area. Also needs instance_id to be a non-zero value.
        const INSTANCED_AREA = 0x080;
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
    pub layer_set_id: u32,
    pub layout_id: u32,
    /// Index into the Weather Excel sheet.
    pub weather_id: u16,
    pub flags: InitZoneFlags,
    pub unk_bitmask1: u8,
    /// Zero means "no obsfucation" (not really, but functionally yes.)
    /// To enable obsfucation, you need to set this to a constant that changes every patch. See lib.rs for the constant.
    pub obsfucation_mode: u8,
    /// First seed used in deobsfucation on the client side.
    pub seed1: u8,
    /// Second seed used in deobsfucation on the client side.
    pub seed2: u8,
    /// Third seed used in deobsfucation on the client size.
    pub seed3: u32,
    pub festival_id: u16,
    pub additional_festival_id: u16,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: [u32; 4],
    pub unk7: [u32; 3],
    pub unk8_9: [u8; 8],
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
