//! Specialized, but overall generic functions for mapping values to bytes should go here.

use bstr::{BString, ByteSlice};
use glam::Vec3;

use crate::common::Position;

pub(crate) fn read_bool_from<T: std::convert::From<u8> + std::cmp::PartialEq>(x: T) -> bool {
    x == T::from(1u8)
}

pub(crate) fn write_bool_as<T: std::convert::From<u8>>(x: &bool) -> T {
    if *x { T::from(1u8) } else { T::from(0u8) }
}

pub(crate) fn read_string(byte_stream: Vec<u8>) -> String {
    read_sestring(byte_stream).to_string()
}

pub(crate) fn write_string(str: &String) -> Vec<u8> {
    write_sestring(&BString::from(str.to_owned()))
}

// TODO: Write an actual SEString parser to replace or wrap usage of BString
// TODO: In the future, maybe expand this to call an internal function that decides from a boolean whether to do SEString parsing or not so we can pass through regular strings as-is (i.e. have read_string call read_sestring(byte_stream, false)).
pub(crate) fn read_sestring(byte_stream: Vec<u8>) -> BString {
    let mut byte_stream = byte_stream;
    byte_stream.push(0); // Guard against streams that don't have a null terminator

    // Find the index of the null terminator.
    let index = byte_stream.iter().position(|b| *b == 0x00).unwrap();

    BString::from(&byte_stream[..index])
}

pub(crate) fn write_sestring(str: &BString) -> Vec<u8> {
    let mut byte_stream: Vec<u8> = str.bytes().collect();

    let index = byte_stream.iter().position(|b| *b == 0x00);

    // If this string doesn't have a null terminator for some reason, add our own.
    if index.is_none() {
        byte_stream.push(0);
    }

    byte_stream
}

/// Converts a quantized rotation to degrees in f32
pub(crate) fn read_quantized_rotation(quantized: u16) -> f32 {
    let max = u16::MAX as f32;
    let pi = std::f32::consts::PI;

    quantized as f32 / max * (2.0 * pi) - pi
}

/// Converts a rotation (in degrees) to
pub(crate) fn write_quantized_rotation(quantized: &f32) -> u16 {
    let max = u16::MAX as f32;
    let pi = std::f32::consts::PI;

    (((quantized + pi) / (2.0 * pi)) * max) as u16
}

pub(crate) fn read_packed_float(packed: u16) -> f32 {
    ((packed as f32 / 0.327675) / 100.0) - 1000.0
}

pub(crate) fn write_packed_float(float: &f32) -> u16 {
    (((float + 1000.0) * 100.0) * 0.327675) as u16
}

pub(crate) fn read_packed_position(packed: [u16; 3]) -> Position {
    Position(Vec3 {
        x: read_packed_float(packed[0]),
        y: read_packed_float(packed[1]),
        z: read_packed_float(packed[2]),
    })
}

pub(crate) fn write_packed_position(pos: &Position) -> [u16; 3] {
    [
        write_packed_float(&pos.0.x),
        write_packed_float(&pos.0.y),
        write_packed_float(&pos.0.z),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: [u8; 2] = [0u8, 1u8];

    #[test]
    fn read_bool_u8() {
        assert!(!read_bool_from::<u8>(DATA[0]));
        assert!(read_bool_from::<u8>(DATA[1]));
    }

    #[test]
    fn write_bool_u8() {
        assert_eq!(write_bool_as::<u8>(&false), DATA[0]);
        assert_eq!(write_bool_as::<u8>(&true), DATA[1]);
    }

    // "FOO\0"
    const STRING_DATA: [u8; 4] = [0x46u8, 0x4Fu8, 0x4Fu8, 0x0u8];

    // "Helper Name" followed by numerous zeroes and garbage data at the end, intended to trip up our old string parser
    const MALFORMED_STRING_DATA: [u8; 32] = [
        0x48, 0x65, 0x6C, 0x70, 0x65, 0x72, 0x20, 0x4E, 0x61, 0x6D, 0x65, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xD8, 0x33,
    ];

    // "Edda Miller" with garbage at the very end of the field
    const MALFORMED_SECOND_EXAMPLE: [u8; 32] = [
        69, 100, 100, 97, 32, 77, 105, 108, 108, 101, 114, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 40, 86,
    ];

    #[test]
    fn read_string() {
        // The nul terminator is supposed to be removed
        assert_eq!(
            crate::common::read_string(STRING_DATA.to_vec()),
            "FOO".to_string()
        );

        // Will this malformed name string trip up our parser? It makes our previous one blow up.
        assert_eq!(
            crate::common::read_string(MALFORMED_STRING_DATA.to_vec()),
            "Helper Name".to_string()
        );

        assert_eq!(
            crate::common::read_string(MALFORMED_SECOND_EXAMPLE.to_vec()),
            "Edda Miller".to_string()
        );
        // TODO: Maybe add SEString tests that can display some printable form of auto-translate phrases after parsing
    }

    #[test]
    fn write_string() {
        // Supposed to include the nul terminator
        assert_eq!(
            crate::common::write_string(&"FOO".to_string()),
            STRING_DATA.to_vec()
        );
    }

    #[test]
    fn quantized_rotations() {
        assert_eq!(read_quantized_rotation(0), -std::f32::consts::PI);
        assert_eq!(read_quantized_rotation(65535), std::f32::consts::PI);

        assert_eq!(write_quantized_rotation(&-std::f32::consts::PI), 0);
        assert_eq!(write_quantized_rotation(&std::f32::consts::PI), 65535);
    }

    #[test]
    fn packed_floats() {
        assert_eq!(read_packed_float(32931), 4.989685);
        assert_eq!(write_packed_float(&5.0), 32931);
    }
}
