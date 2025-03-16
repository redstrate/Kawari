use std::io::Cursor;

use binrw::BinResult;

use crate::blowfish::Blowfish;

use super::IpcSegmentTrait;

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
pub(crate) fn decrypt<T: IpcSegmentTrait>(
    size: u32,
    encryption_key: Option<&[u8]>,
) -> BinResult<T> {
    if let Some(encryption_key) = encryption_key {
        let size = size - (std::mem::size_of::<u32>() * 4) as u32; // 16 = header size

        let mut data = vec![0; size as usize];
        reader.read_exact(&mut data)?;

        let blowfish = Blowfish::new(encryption_key);
        blowfish.decrypt(&mut data);

        let mut cursor = Cursor::new(&data);
        T::read_options(&mut cursor, endian, ())
    } else {
        tracing::info!("NOTE: Not decrypting this IPC packet since no key was provided!");

        T::read_options(reader, endian, ())
    }
}

#[binrw::writer(writer, endian)]
pub(crate) fn encrypt<T: IpcSegmentTrait>(
    value: &T,
    size: u32,
    encryption_key: Option<&[u8]>,
) -> BinResult<()> {
    if let Some(encryption_key) = encryption_key {
        let size = size - (std::mem::size_of::<u32>() * 4) as u32; // 16 = header size

        let mut cursor = Cursor::new(Vec::new());
        value.write_options(&mut cursor, endian, ())?;

        let mut buffer = cursor.into_inner();
        buffer.resize(size as usize, 0);

        let blowfish = Blowfish::new(encryption_key);
        blowfish.encrypt(&mut buffer);

        writer.write_all(&buffer)?;

        Ok(())
    } else {
        tracing::info!("NOTE: Not encrypting this IPC packet since no key was provided!");

        value.write_options(writer, endian, ())
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
