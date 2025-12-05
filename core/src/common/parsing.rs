//! Specialized, but overall generic functions for mapping values to bytes should go here.

use std::ffi::{CStr, CString};

use crate::common::Position;

pub(crate) fn read_bool_from<T: std::convert::From<u8> + std::cmp::PartialEq>(x: T) -> bool {
    x == T::from(1u8)
}

pub(crate) fn write_bool_as<T: std::convert::From<u8>>(x: &bool) -> T {
    if *x { T::from(1u8) } else { T::from(0u8) }
}

pub(crate) fn read_string(byte_stream: Vec<u8>) -> String {
    // TODO: This can surely be made better, but it seems to satisfy some strange edge cases. If there are even more found, then we should probably rewrite this function altogether.
    let Ok(result) = CStr::from_bytes_until_nul(&byte_stream) else {
        if let Ok(str) = String::from_utf8(byte_stream.clone()) {
            return str.trim_matches(char::from(0)).to_string(); // trim \0 from the end of strings
        } else {
            tracing::error!(
                "Found an edge-case where both CStr::from_bytes_until_nul and String::from_utf8 failed: {:#?}",
                byte_stream.clone()
            );
            return String::default();
        }
    };

    let Ok(result) = result.to_str().to_owned() else {
        tracing::error!("Unable to make this CStr an owned string, what happened?");
        return String::default();
    };

    result.to_string()
}

pub(crate) fn write_string(str: &String) -> Vec<u8> {
    let c_string = CString::new(&**str).unwrap();
    c_string.as_bytes_with_nul().to_vec()
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
    Position {
        x: read_packed_float(packed[0]),
        y: read_packed_float(packed[1]),
        z: read_packed_float(packed[2]),
    }
}

pub(crate) fn write_packed_position(pos: &Position) -> [u16; 3] {
    [
        write_packed_float(&pos.x),
        write_packed_float(&pos.y),
        write_packed_float(&pos.z),
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
