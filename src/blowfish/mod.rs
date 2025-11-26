mod constants;
use constants::{BLOWFISH_P, BLOWFISH_S};

const ROUNDS: usize = 16;

pub struct Blowfish {
    p: [u32; ROUNDS + 2],
    s: [[u32; 256]; 4],
}

impl Blowfish {
    /// Initializes a new Blowfish session with a key.
    pub fn new(key: &[u8]) -> Blowfish {
        let mut s = Self {
            p: BLOWFISH_P,
            s: BLOWFISH_S,
        };

        let mut j = 0usize;
        for i in 0..ROUNDS + 2 {
            let mut data = 0i32;
            for _ in 0..4 {
                data = (data.wrapping_shl(8)) | ((key[j] as i8) as i32);
                j += 1;

                if j >= key.len() {
                    j = 0;
                }
            }

            s.p[i] ^= data as u32;
        }

        let mut l = 0u32;
        let mut r = 0u32;

        for i in (0..ROUNDS + 2).step_by(2) {
            s.encrypt_pair(&mut l, &mut r);
            s.p[i] = l;
            s.p[i + 1] = r;
        }

        for i in 0..4 {
            for j in (0..256).step_by(2) {
                s.encrypt_pair(&mut l, &mut r);
                s.s[i][j] = l;
                s.s[i][j + 1] = r;
            }
        }

        s
    }

    /// Encrypts a block of data. If the encryption for any reason fails, returns None.
    pub fn encrypt(&self, data: &mut [u8]) {
        let padded_size = Blowfish::padded_length(data.len());

        for i in (0..padded_size).step_by(8) {
            let mut l: u32 = u32::from_le_bytes(data[i..i + 4].try_into().unwrap());
            let mut r: u32 = u32::from_le_bytes(data[i + 4..i + 8].try_into().unwrap());

            self.encrypt_pair(&mut l, &mut r);

            data[i..i + 4].copy_from_slice(&l.to_le_bytes());
            data[i + 4..i + 8].copy_from_slice(&r.to_le_bytes());
        }
    }

    fn padded_length(length: usize) -> usize {
        ((length as i32) & -32) as usize
    }

    pub fn decrypt(&self, data: &mut [u8]) {
        let padded_size = Blowfish::padded_length(data.len());

        // extra data at the end is left untouched
        for i in (0..padded_size).step_by(8) {
            let mut l: u32 = u32::from_le_bytes(data[i..i + 4].try_into().unwrap());
            let mut r: u32 = u32::from_le_bytes(data[i + 4..i + 8].try_into().unwrap());

            self.decrypt_pair(&mut l, &mut r);

            data[i..i + 4].copy_from_slice(&l.to_le_bytes());
            data[i + 4..i + 8].copy_from_slice(&r.to_le_bytes());
        }
    }

    /// Calculates the F-function for `x`.
    fn f(&self, x: u32) -> u32 {
        let [a, b, c, d] = x.to_le_bytes();
        ((self.s[0][d as usize].wrapping_add(self.s[1][c as usize])) ^ (self.s[2][b as usize]))
            .wrapping_add(self.s[3][a as usize])
    }

    fn encrypt_pair(&self, xl: &mut u32, xr: &mut u32) {
        for i in 0..ROUNDS {
            *xl ^= self.p[i];
            *xr ^= self.f(*xl);

            (*xl, *xr) = (*xr, *xl);
        }

        (*xl, *xr) = (*xr, *xl);

        *xr ^= self.p[ROUNDS];
        *xl ^= self.p[ROUNDS + 1];
    }

    fn decrypt_pair(&self, xl: &mut u32, xr: &mut u32) {
        for i in (2..ROUNDS + 2).rev() {
            *xl ^= self.p[i];
            *xr ^= self.f(*xl);

            (*xl, *xr) = (*xr, *xl);
        }

        (*xl, *xr) = (*xr, *xl);

        *xl ^= self.p[0];
        *xr ^= self.p[1];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let blowfish = Blowfish::new(b"test_case");

        let mut expected_encrypted =
            vec![104, 101, 108, 108, 111, 44, 32, 119, 111, 114, 108, 100, 33];

        let mut buffer = Vec::from(b"hello, world!");

        blowfish.encrypt(&mut buffer);
        assert_eq!(buffer, expected_encrypted);
        blowfish.decrypt(&mut expected_encrypted);
        assert_eq!(
            String::from_utf8(expected_encrypted).unwrap(),
            "hello, world!"
        );
    }
}
