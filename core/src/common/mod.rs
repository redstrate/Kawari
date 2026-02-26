#[cfg(test)]
use binrw::BinWrite;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod customize_data;
pub use customize_data::CustomizeData;

mod position;
pub use position::Position;

mod director;
pub use director::*;

mod game;
pub use game::*;

mod object;
pub use object::*;

mod parsing;
pub(crate) use parsing::*;

mod dropin;
pub use dropin::*;

/// Get the number of seconds since UNIX epoch.
pub fn timestamp_secs() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get UNIX timestamp!")
        .as_secs()
        .try_into()
        .unwrap()
}

/// Get the number of milliseconds since UNIX epoch.
pub fn timestamp_msecs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get UNIX timestamp!")
        .as_millis()
        .try_into()
        .unwrap()
}

/// Gets the initial zone for a given city-state id
pub fn determine_initial_starting_zone(citystate_id: u8) -> u16 {
    match citystate_id {
        // Limsa
        1 => 181,
        // Gridania
        2 => 183,
        // Ul'dah
        3 => 182,
        _ => panic!("This is not a valid city-state id!"),
    }
}

pub fn value_to_flag_byte_index_value(in_value: u32) -> (u8, u16) {
    let bit_index = in_value % 8;
    (1 << bit_index, (in_value / 8) as u16)
}

pub fn value_to_flag_byte_index_value_quests(in_value: u32) -> (u8, u16) {
    let bit_index = in_value % 8;
    (0x80 >> bit_index, (in_value / 8) as u16)
}

// Just based off of Sapphire's version
pub fn euler_to_direction(euler: [f32; 3]) -> f32 {
    let sin_z = f32::sin(euler[2]);
    let cos_z = f32::cos(euler[2]);
    let sin_y = f32::sin(euler[1]);
    let cos_y = f32::cos(euler[1]);
    let sin_x = f32::sin(euler[0]);
    let cos_x = f32::cos(euler[0]);

    let m00 = cos_z * cos_y;
    let m02 = sin_z * sin_x + (-cos_z * sin_y) * cos_x;

    let m10 = -sin_z * cos_y;
    let m12 = cos_z * sin_x + sin_z * sin_y * cos_x;

    let m20 = sin_y;
    let m22 = cos_y * cos_x;

    let vector = [0.0, 0.0, 1.0];
    let dst_x = vector[2] * m20 + vector[0] * m00 + vector[1] * m10;
    let dst_z = vector[2] * m22 + vector[0] * m02 + vector[1] * m12;

    let squared = dst_z * dst_z + dst_x * dst_x;
    let v1;
    let v2;

    if squared > 0.00000011920929 {
        let mut ret = f32::sqrt(squared);
        ret = -((squared * ret) * ret - 1.0) * (0.5 * ret) + ret;
        ret = -((squared * ret) * ret - 1.0) * (0.5 * ret) + ret;
        v1 = dst_z * (-(((squared * ret) * ret) - 1.0) * (0.5 * ret) + ret);
        v2 = dst_x * (-(((squared * ret) * ret) - 1.0) * (0.5 * ret) + ret);
    } else {
        v1 = 0.0;
        v2 = 0.0;
    }

    f32::atan2(v2, v1)
}

/// Calculates the maximum achievable level for a given expansion.
/// For example, 0 (ARR) would be 50.
pub fn calculate_max_level(expansion: u8) -> u8 {
    50 + (expansion * 10)
}

/// The maximum size of our packet buffers, anything bigger than this from the client is truncated.
pub const RECEIVE_BUFFER_SIZE: usize = 0xFFFF;

/// Error messages: TODO: this should probably be moved into its own universal mod/crate?
pub const ERR_INVENTORY_ADD_FAILED: &str =
    "Unable to add item to inventory! Your inventory is full, or this is a bug in Kawari!";

/// Service name for the account management pages. This is used to uniquely identify sessions.
pub const ACCOUNT_MANAGEMENT_SERVICE: &str = "Kawari: Account Management";

/// Service name for game logins. This is used to uniquely identify sessions.
pub const GAME_SERVICE: &str = "Kawari: Game Client";

/// Timeout in seconds before clients are disconnected because of idle network activity.
pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(5);

/// Name of the World used in certain scenarios.
pub const WORLD_NAME: &str = "Kawari";

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct BasicCharacterData {
    pub content_id: u64,
    pub name: String,
}

/// Turns a Quest row ID into a "normal" one. For example: 65537 to 1.
pub fn adjust_quest_id(quest_id: u32) -> u32 {
    quest_id.saturating_sub(65536)
}

#[macro_export]
macro_rules! web_static_dir {
    ($rel_path:literal) => {
        concat!("resources/web/static/", $rel_path)
    };
}

/// Helper to automatically test each opcode to ensure it matches the expected size.
/// This only ensures we aren't buggy writing up to that size, if we were wrong about the size this will still pass.
#[cfg(test)]
pub fn test_opcodes<Segment: crate::packet::ReadWriteIpcSegment>() {
    use crate::packet::{HasUnknownData, ReadWriteIpcOpcode};

    let ipc_types = Segment::Data::create_default_variants();

    for data in ipc_types {
        let mut cursor = std::io::Cursor::new(Vec::new());

        let opcode = Segment::OpCode::from_data(&data);
        let ipc_segment = Segment::new(crate::packet::IpcSegmentHeader::from_opcode(opcode), data);
        ipc_segment.write_le(&mut cursor).unwrap();

        let buffer = cursor.into_inner();

        let opcode_name = ipc_segment.get_name();
        assert_eq!(
            buffer.len(),
            ipc_segment.calc_size() as usize,
            "{opcode_name} did not match size!"
        );
    }
}

/// Helper to ensure that type `T` is written to `EXPECTED_SIZE`.
#[cfg(test)]
pub fn ensure_size<T: BinWrite + Default, const EXPECTED_SIZE: usize>()
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'a + Default,
{
    use std::io::Cursor;

    let mut cursor = Cursor::new(Vec::new());
    let instance = T::default();
    instance.write_ne(&mut cursor).expect("Failed to write!");

    assert_eq!(cursor.position() as usize, EXPECTED_SIZE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_flag() {
        assert_eq!(value_to_flag_byte_index_value(0), (1, 0));
        assert_eq!(value_to_flag_byte_index_value(32), (1, 4));
        assert_eq!(value_to_flag_byte_index_value(64), (1, 8));
    }

    #[test]
    fn test_max_ex_levels() {
        assert_eq!(calculate_max_level(0), 50); // A Realm Reborn
        assert_eq!(calculate_max_level(1), 60); // Heavensward
        assert_eq!(calculate_max_level(2), 70); // Stormblood
        assert_eq!(calculate_max_level(3), 80); // Shadowbringers
        assert_eq!(calculate_max_level(4), 90); // Endwalker
        assert_eq!(calculate_max_level(5), 100); // Dawntrail
    }

    #[test]
    fn test_euler_to_direction() {
        assert_eq!(euler_to_direction([0.0, 0.0, 0.0]), 0.0);
        assert_eq!(euler_to_direction([90.0, 0.0, 0.0]), 3.1415927);
        assert_eq!(euler_to_direction([0.0, 90.0, 0.0]), 2.0354056);
        assert_eq!(euler_to_direction([0.0, 0.0, 90.0]), 0.0);
        assert_eq!(euler_to_direction([-90.0, 0.0, 0.0]), 3.1415927);
        assert_eq!(euler_to_direction([0.0, -90.0, 0.0]), -2.0354056);
        assert_eq!(euler_to_direction([0.0, 0.0, -90.0]), 0.0);
    }

    // Helper macros so we don't repeat ourselves in tests a bunch of times
    #[macro_export]
    macro_rules! client_zone_tests_dir {
        ($rel_path:literal) => {
            concat!("../resources/data/tests/zone/client/", $rel_path)
        };
    }

    #[macro_export]
    macro_rules! server_zone_tests_dir {
        ($rel_path:literal) => {
            concat!("../resources/data/tests/zone/server/", $rel_path)
        };
    }

    #[macro_export]
    macro_rules! client_chat_tests_dir {
        ($rel_path:literal) => {
            concat!("../resources/data/tests/chat/client/", $rel_path)
        };
    }

    #[macro_export]
    macro_rules! server_chat_tests_dir {
        ($rel_path:literal) => {
            concat!("../resources/data/tests/chat/server/", $rel_path)
        };
    }

    // Helper macros so we don't repeat ourselves a bunch of times
    #[macro_export]
    macro_rules! client_lobby_tests_dir {
        ($rel_path:literal) => {
            concat!("../resources/data/tests/lobby/client/", $rel_path)
        };
    }

    #[macro_export]
    macro_rules! server_lobby_tests_dir {
        ($rel_path:literal) => {
            concat!("../resources/data/tests/lobby/server/", $rel_path)
        };
    }
}
