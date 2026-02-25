use binrw::binrw;
use strum_macros::{Display, EnumIter, FromRepr};

use crate::common::{HandlerId, ObjectTypeId};

/// See the EventSystemDefine Excel Sheet.
#[binrw]
#[brw(repr = u8)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Display, EnumIter, FromRepr)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    /// Used when talking to NPCs.
    #[default]
    Talk = 1,
    Emote = 2,
    DistanceBelow = 3,
    DistanceOver = 4,
    BattleReward = 5,
    Craft = 6,
    /// Used when events want to nest another event.
    Nest = 7,
    EventItem = 8,
    Drop = 9,
    /// Used for gimmick path events?
    WithinRange = 10,
    OutsideRange = 11,
    GameStart = 12,
    /// Seen in Gold Saucer Invitational
    GameProgress = 13,
    /// Unknown?
    EnterTerritory = 15,
    GameComeBack = 17,
    ActionResult = 18,
    MateriaCraft = 19,
    /// Seen while fishing.
    Fishing = 20,
    UI = 21,
    Housing = 22,
    Say = 23,
    TableGame = 24,
}

#[cfg(feature = "server")]
impl mlua::IntoLua for EventType {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct EventStart {
    pub target_id: ObjectTypeId,
    pub handler_id: HandlerId,
    pub event_type: EventType,
    pub flags: u8,
    #[brw(pad_before = 2)]
    #[brw(pad_after = 4)]
    pub event_arg: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::common::{HandlerType, ObjectId, ObjectTypeKind};

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_intro_event_start() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("event_start.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let event_start = EventStart::read_le(&mut buffer).unwrap();
        assert_eq!(
            event_start.target_id,
            ObjectTypeId {
                object_id: ObjectId(277124129),
                object_type: ObjectTypeKind::None,
            }
        );
        assert_eq!(event_start.handler_id, HandlerId(0x130003)); // aether intro
        assert_eq!(event_start.event_type, EventType::EnterTerritory);
        assert_eq!(event_start.flags, 0);
        assert_eq!(event_start.event_arg, 182);
    }

    #[test]
    fn read_gold_saucer_invitational() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("event_start_invitational.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let event_start = EventStart::read_le(&mut buffer).unwrap();
        assert_eq!(
            event_start.target_id,
            ObjectTypeId {
                object_id: ObjectId(276833873),
                object_type: ObjectTypeKind::None,
            }
        );
        assert_eq!(
            event_start.handler_id,
            HandlerId::new(HandlerType::InstanceContent, 26002)
        );
        assert_eq!(event_start.event_type, EventType::GameProgress);
        assert_eq!(event_start.flags, 1);
        assert_eq!(event_start.event_arg, 8);
    }

    #[test]
    fn read_fishing() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("event_start_fishing.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let event_start = EventStart::read_le(&mut buffer).unwrap();
        assert_eq!(
            event_start.target_id,
            ObjectTypeId {
                object_id: ObjectId(277869081),
                object_type: ObjectTypeKind::None,
            }
        );
        assert_eq!(
            event_start.handler_id,
            HandlerId::new(HandlerType::Fishing, 1)
        );
        assert_eq!(event_start.event_type, EventType::Fishing);
        assert_eq!(event_start.flags, 0);
        assert_eq!(event_start.event_arg, 0);
    }
}
