use std::io::Cursor;

use binrw::BinResult;

use crate::{GAME_VERSION, blowfish::Blowfish};

use super::{IPC_HEADER_SIZE, ReadWriteIpcSegment, parsing::ConnectionState};

pub fn generate_encryption_key(key: &[u8], phrase: &str) -> [u8; 16] {
    let mut base_key = vec![0x78, 0x56, 0x34, 0x12];
    base_key.extend_from_slice(key);
    base_key.extend_from_slice(&GAME_VERSION.to_le_bytes());
    base_key.extend_from_slice(phrase.as_bytes());

    md5::compute(&base_key).0
}

#[binrw::parser(reader, endian)]
pub(crate) fn decrypt<T: ReadWriteIpcSegment>(size: u32, state: &ConnectionState) -> BinResult<T> {
    if let ConnectionState::Lobby { client_key } = state {
        let size = size - IPC_HEADER_SIZE;

        let mut data = vec![0; size as usize];
        reader.read_exact(&mut data)?;

        let blowfish = Blowfish::new(client_key);
        blowfish.decrypt(&mut data);

        let mut cursor = Cursor::new(&data);
        T::read_options(&mut cursor, endian, (&size,))
    } else {
        T::read_options(reader, endian, (&size,))
    }
}

#[binrw::writer(writer, endian)]
pub(crate) fn encrypt<T: ReadWriteIpcSegment>(
    value: &T,
    size: u32,
    state: &ConnectionState,
) -> BinResult<()> {
    if let ConnectionState::Lobby { client_key } = state {
        let size = size - IPC_HEADER_SIZE;

        let mut cursor = Cursor::new(Vec::new());
        value.write_options(&mut cursor, endian, ())?;

        let mut buffer = cursor.into_inner();
        buffer.resize(size as usize, 0);

        let blowfish = Blowfish::new(client_key);
        blowfish.encrypt(&mut buffer);

        writer.write_all(&buffer)?;

        Ok(())
    } else {
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
                227, 149, 193, 76, 138, 70, 97, 23, 16, 47, 127, 153, 97, 109, 29, 87
            ]
        );
    }
}
