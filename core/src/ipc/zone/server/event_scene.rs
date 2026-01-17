#![allow(unused_assignments)] // false positive caused by binrw

use binrw::binrw;
use bitflags::bitflags;

use crate::common::{HandlerId, ObjectTypeId};
use crate::ipc::zone::server::{ServerZoneIpcData, ServerZoneIpcSegment};

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SceneFlags(pub u32);

impl std::fmt::Debug for SceneFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

impl std::fmt::Display for SceneFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

// From https://github.com/SapphireServer/Sapphire/blob/a203ffef937df84c50c7fec130f2acd39efdd31e/src/world/Event/EventDefs.h#L19
// TODO: should be documented which each of these do
bitflags! {
    impl SceneFlags : u32 {
        // Due to some bitflags crate nonsense, these combination flags need to be declared first.
        const SET_BASE = Self::NO_DEFAULT_CAMERA.bits() | Self::FADE_OUT.bits() | Self::INVIS_EOBJ.bits() | Self::INVIS_BNPC.bits() | Self::INVIS_OTHER_PC.bits() | Self::INVIS_PARTY_PC.bits() | Self::INVIS_PARTY_BUDDY.bits() | Self::INVIS_GATHERING_POINT.bits() | Self::INVIS_TREASURE.bits() | Self::CONDITION_CUTSCENE.bits() | Self::HIDE_UI.bits() | Self::DISABLE_STEALTH.bits() | Self::INVIS_AOE.bits() | Self::INVIS_ALLIANCE_PC.bits() | Self::INVIS_ALLIANCE_BUDDY.bits() | Self::INVIS_COMPANION.bits();

        const NONE = 0x00000000;
        const NO_DEFAULT_CAMERA = 0x00000001;
        const FADE_OUT = 0x00000002;
        const INVIS_ENPC = 0x00000004;
        const INVIS_EOBJ = 0x00000008;
        const INVIS_BNPC = 0x00000010;
        const INVIS_OTHER_PC = 0x00000020;
        const INVIS_PARTY_PC = 0x00000040;
        const INVIS_PARTY_BUDDY = 0x10000000;
        const INVIS_GATHERING_POINT = 0x00000080;
        const INVIS_AETHERYTE = 0x00000100;
        const INVIS_TREASURE = 0x00000200;
        const CONDITION_CUTSCENE = 0x00000400;
        const HIDE_UI = 0x00000800;
        const INVIS_ALL = 0xF80003FC;
        const AUTO_LOC_CAMERA = 0x00001000;
        const HIDE_HOTBAR = 0x00002000;
        const INVINCIBLE = 0x00004000;
        const SILENT_ENTER_TERRI_ENV = 0x00008000;
        const SILENT_ENTER_TERRI_BGM = 0x00010000;
        const SILENT_ENTER_TERRI_SE = 0x00020000;
        const SILENT_ENTER_TERRI_ALL = 0x00038000;
        const DISABLE_SKIP = 0x00080000;
        const HIDE_FESTIVAL = 0x00200000;
        const DISABLE_STEALTH = 0x00400000;
        const ROLLBACK_HIDE_UI = 0x00800000;
        const LOCK_HUD = 0x01000000;
        const LOCK_HOTBAR = 0x02000000;
        const DISABLE_CANCEL_EMOTE = 0x04000000;
        const INVIS_AOE = 0x08000000;
        const INVIS_ALLIANCE_PC = 0x20000000;
        const INVIS_ALLIANCE_BUDDY = 0x40000000;
        const INVIS_COMPANION = 0x80000000;
    }
}

impl Default for SceneFlags {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, Default)]
#[binrw]
#[brw(little)]
#[brw(import{max_params: usize})]
#[brw(assert(params.len() <= max_params, "Too many params! {} > {}", params.len(), max_params))]
pub struct EventScene {
    pub actor_id: ObjectTypeId,
    pub handler_id: HandlerId,
    pub scene: u16,
    #[brw(pad_before = 2)] // FIXME: um, i don't think this is empty!!
    pub scene_flags: SceneFlags,
    pub unk1: u32,
    pub params_count: u8,
    // Extra padding seems needed after or the client will seemingly softlock even with 2 params, possibly used for alignment?
    #[brw(pad_before = 3, pad_after = 4)]
    #[br(count = max_params)]
    #[bw(pad_size_to = 4 * max_params)]
    pub params: Vec<u32>,
}

impl EventScene {
    pub fn package_scene(&self) -> Option<ServerZoneIpcSegment> {
        match self.params.len() {
            // TODO: it would be nice to avoid cloning if possible
            0..=2 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::EventScene {
                data: self.clone(),
            })),
            3..=4 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::EventScene4 {
                data: self.clone(),
            })),
            5..=8 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::EventScene8 {
                data: self.clone(),
            })),
            9..=16 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::EventScene16 {
                data: self.clone(),
            })),
            17..=32 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::EventScene32 {
                data: self.clone(),
            })),
            33..=64 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::EventScene64 {
                data: self.clone(),
            })),
            65..=128 => Some(ServerZoneIpcSegment::new(
                ServerZoneIpcData::EventScene128 { data: self.clone() },
            )),
            129..=255 => Some(ServerZoneIpcSegment::new(
                ServerZoneIpcData::EventScene255 { data: self.clone() },
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::common::{ObjectId, ObjectTypeKind};

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_intro_event_start() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("event_play.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let event_play =
            EventScene::read_le_args(&mut buffer, EventSceneBinReadArgs { max_params: 2 }).unwrap();
        assert_eq!(
            event_play.actor_id,
            ObjectTypeId {
                object_id: ObjectId(277124129),
                object_type: ObjectTypeKind::None,
            }
        );
        assert_eq!(event_play.handler_id, HandlerId(0x130003)); // aether intro
        assert_eq!(event_play.scene, 0);
        assert_eq!(
            event_play.scene_flags,
            SceneFlags::NO_DEFAULT_CAMERA
                | SceneFlags::INVIS_ENPC
                | SceneFlags::CONDITION_CUTSCENE
                | SceneFlags::HIDE_UI
                | SceneFlags::HIDE_HOTBAR
                | SceneFlags::SILENT_ENTER_TERRI_ENV
                | SceneFlags::SILENT_ENTER_TERRI_BGM
                | SceneFlags::SILENT_ENTER_TERRI_SE
                | SceneFlags::DISABLE_SKIP
                | SceneFlags::DISABLE_STEALTH
        );
        assert_eq!(event_play.unk1, 0);
        assert_eq!(event_play.params_count, 1);
        assert_eq!(event_play.params[0], 0);
        assert_eq!(event_play.params[1], 0);
    }
}
