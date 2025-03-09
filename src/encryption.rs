use std::fs::write;
use std::{io::Cursor, slice};

use binrw::{BinRead, BinResult, BinWrite};

#[link(name = "FFXIVBlowfish")]
unsafe extern "C" {
    pub fn blowfish_encode(
        key: *const u8,
        keybytes: u32,
        pInput: *const u8,
        lSize: u32,
    ) -> *const u8;
    pub fn blowfish_decode(
        key: *const u8,
        keybytes: u32,
        pInput: *const u8,
        lSize: u32,
    ) -> *const u8;
}

const GAME_VERSION: u16 = 7000;

pub fn generate_encryption_key(key: &[u8], phrase: &str) -> [u8; 16] {
    let mut base_key = vec![0x78, 0x56, 0x34, 0x12];
    base_key.extend_from_slice(key);
    base_key.extend_from_slice(&GAME_VERSION.to_le_bytes());
    base_key.extend_from_slice(&[0; 2]); // padding (possibly for game version?)
    base_key.extend_from_slice(phrase.as_bytes());

    md5::compute(&base_key).0
}

#[binrw::parser(reader, endian)]
pub(crate) fn decrypt<T>(size: u32, encryption_key: Option<&[u8]>) -> BinResult<T>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'a,
{
    let Some(encryption_key) = encryption_key else {
        panic!("This segment type is encrypted and no key was provided!");
    };

    let size = size - (std::mem::size_of::<u32>() * 4) as u32; // 16 = header size

    let mut data = vec![0; size as usize];
    reader.read_exact(&mut data)?;

    unsafe {
        let decryption_result = blowfish_decode(encryption_key.as_ptr(), 16, data.as_ptr(), size);
        let decrypted_data = slice::from_raw_parts(decryption_result, size as usize);

        write("decrypted.bin", decrypted_data).unwrap();

        let mut cursor = Cursor::new(&decrypted_data);
        T::read_options(&mut cursor, endian, ())
    }
}

#[binrw::writer(writer, endian)]
pub(crate) fn encrypt<T>(value: &T, size: u32, encryption_key: Option<&[u8]>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'a,
{
    let Some(encryption_key) = encryption_key else {
        panic!("This segment type needs to be encrypted and no key was provided!");
    };

    let size = size - (std::mem::size_of::<u32>() * 4) as u32; // 16 = header size

    let mut cursor = Cursor::new(Vec::new());
    value.write_options(&mut cursor, endian, ())?;

    let mut buffer = cursor.into_inner();
    buffer.resize(size as usize, 0);

    unsafe {
        let encoded = blowfish_encode(
            encryption_key.as_ptr(),
            16,
            buffer.as_ptr(),
            buffer.len() as u32,
        );
        let encoded_data = slice::from_raw_parts(encoded, size as usize);
        writer.write_all(encoded_data)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_key() {
        let key = generate_encryption_key(&[0x00, 0x00, 0x00, 0x00], "foobar");
        assert_eq!(
            key,
            [
                169, 78, 235, 31, 57, 151, 26, 74, 250, 196, 1, 120, 206, 173, 202, 48
            ]
        );
    }
}
