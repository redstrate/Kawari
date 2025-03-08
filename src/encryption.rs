use std::io::Cursor;
use std::fs::write;

use binrw::{BinRead, BinResult};
use physis::blowfish::Blowfish;

const GAME_VERSION: u16 = 7000;

pub fn generate_encryption_key(key: &[u8], phrase: &str) -> [u8; 16] {
    let mut base_key = vec![0x78, 0x56, 0x34, 0x12];
    base_key.extend_from_slice(&key);
    base_key.extend_from_slice(&GAME_VERSION.to_le_bytes());
    base_key.extend_from_slice(&[0; 2]); // padding (possibly for game version?)
    base_key.extend_from_slice(&phrase.as_bytes());

    md5::compute(&base_key).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_key() {
        let key = generate_encryption_key([0x00, 0x00, 0x00, 0x00], "foobar");
        assert_eq!(
            key,
            [
                169, 78, 235, 31, 57, 151, 26, 74, 250, 196, 1, 120, 206, 173, 202, 48
            ]
        );
    }
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

    let mut data = Vec::new();
    data.resize(size as usize, 0x0);
    reader.read_exact(&mut data)?;

    write("encrypted.bin", &data);

    let blowfish = Blowfish::new(encryption_key);
    let decrypted_data = blowfish.decrypt(&data).unwrap();

    write("decrypted.bin", &decrypted_data);

    let mut cursor = Cursor::new(&decrypted_data);
    T::read_options(&mut cursor, endian, ())
}
