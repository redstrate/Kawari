use binrw::binrw;

use crate::common::{ObjectTypeId, read_quantized_rotation, write_quantized_rotation};

/// See <https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Game/ActionManager.cs#L395>
#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, Copy, Default)]
#[brw(repr = u8)]
pub enum ActionType {
    #[default]
    None,
    Action,
    Item,
    EventItem,
    EventAction,
    GeneralAction,
    BuddyAction,
    MainCommand,
    Companion,
    CraftAction,
    Unk10, // Fishing per Sapphire? Something to do with items.
    PetAction,
    Unk12, // Not in UseAction. Sapphire says CompanyAction, but not actually triggered.
    Mount,
    PvPAction,
    FieldMarker,
    ChocoboRaceAbility,
    ChocoboRaceItem,
    Unk18, // Not in UseAction (?)
    BgcArmyAction,
    Ornament,
}

#[binrw]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActionRequest {
    /// Index into the Action Excel sheet.
    pub action_id: u32,
    pub unk1: u8, // what?
    /// What kind of action is requested.
    pub action_type: ActionType,
    /// Will show up again in the resulting `ActionResult`.
    pub sequence: u16,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation1: f32,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation2: f32,
    pub unk3: u16,
    pub unk4: u16,
    pub target: ObjectTypeId,
    pub arg: u32,
    pub padding_prob: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::common::ObjectId;

    use crate::client_zone_tests_dir;

    use super::*;

    #[test]
    fn read_actionrequest() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(client_zone_tests_dir!("action_request.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_request = ActionRequest::read_le(&mut buffer).unwrap();
        assert_eq!(action_request.action_type, ActionType::Action);
        assert_eq!(action_request.target.object_id, ObjectId(0x400097d0));
        assert_eq!(action_request.rotation1, -3.141401);
        assert_eq!(action_request.rotation2, 1.9694216);
    }

    #[test]
    fn read_actionrequest_mount() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(client_zone_tests_dir!("action_request_mount.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_request = ActionRequest::read_le(&mut buffer).unwrap();
        assert_eq!(action_request.action_type, ActionType::Mount);
        assert_eq!(action_request.action_id, 55);
        assert_eq!(action_request.target.object_id, ObjectId(277114100));
        assert_eq!(action_request.rotation1, -3.1412091);
        assert_eq!(action_request.rotation2, -0.8154669);
    }
}
