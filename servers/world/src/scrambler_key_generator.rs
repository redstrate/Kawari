use kawari::{constants::{OBFUSCATION_TABLE_MAX, OBFUSCATION_TABLE_RADIXES}, packet::ScramblerKeys};

// Helper macro so we don't repeat ourselves a bunch of times
macro_rules! scrambler_dir {
    ($rel_path:literal) => {
        concat!("../../../resources/data/scrambler/", $rel_path)
    };
}

/// Generates the necessary keys from three seeds.
pub struct ScramblerKeyGenerator {
    table0: Vec<i32>,
    table1: Vec<i32>,
    table2: Vec<i32>,
    mid_table: &'static [u8],
    day_table: &'static [u8],
    opcode_key_table: Vec<i32>,
    table_radixes: &'static [i32],
    table_max: &'static [i32],
}

impl Default for ScramblerKeyGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ScramblerKeyGenerator {
    pub fn new() -> Self {
        Self {
            table0: include_bytes!(scrambler_dir!("table0.bin"))
                .chunks(4)
                .map(|x| i32::from_le_bytes(x.try_into().unwrap()))
                .collect(),
            table1: include_bytes!(scrambler_dir!("table1.bin"))
                .chunks(4)
                .map(|x| i32::from_le_bytes(x.try_into().unwrap()))
                .collect(),
            table2: include_bytes!(scrambler_dir!("table2.bin"))
                .chunks(4)
                .map(|x| i32::from_le_bytes(x.try_into().unwrap()))
                .collect(),
            mid_table: include_bytes!(scrambler_dir!("midtable.bin")),
            day_table: include_bytes!(scrambler_dir!("daytable.bin")),
            opcode_key_table: include_bytes!(scrambler_dir!("opcodekeytable.bin"))
                .chunks(4)
                .map(|x| i32::from_le_bytes(x.try_into().unwrap()))
                .collect(),
            table_radixes: &OBFUSCATION_TABLE_RADIXES,
            table_max: &OBFUSCATION_TABLE_MAX,
        }
    }

    fn derive(&self, set: u8, n_seed_1: u8, n_seed_2: u8, epoch: u32) -> u8 {
        // FIXME: so many probably unnecessary casts here

        let mid_index = 8 * (n_seed_1 as usize % (self.mid_table.len() / 8));
        let mid_table_value = self.mid_table[4 + mid_index];
        let mut mid_bytes = [0u8; 4];
        mid_bytes.copy_from_slice(&self.mid_table[mid_index..mid_index + 4]);
        let mid_value = u32::from_le_bytes(mid_bytes);

        let epoch_days = 3 * (epoch as usize / 60 / 60 / 24);
        let day_table_index = 4 * (epoch_days % (self.day_table.len() / 4));
        let day_table_value = self.day_table[day_table_index];

        let set_radix = self.table_radixes[set as usize];
        let set_max = self.table_max[set as usize];
        let table_index = (set_radix * (n_seed_2 as i32 % set_max)) as usize
            + mid_value as usize * n_seed_1 as usize % set_radix as usize;
        let set_result = match set {
            0 => self.table0[table_index],
            1 => self.table1[table_index],
            2 => self.table2[table_index],
            _ => 0,
        };

        (n_seed_1 as i32 + mid_table_value as i32 + day_table_value as i32 + set_result) as u8
    }

    /// Generates keys for scrambling or unscrambling packets. The callee must keep track of their seeds, we only generate the keys.
    pub fn generate(&self, seed1: u8, seed2: u8, seed3: u32) -> ScramblerKeys {
        let neg_seed_1 = seed1;
        let neg_seed_2 = seed2;
        let neg_seed_3 = seed3;

        ScramblerKeys {
            keys: [
                self.derive(0, neg_seed_1, neg_seed_2, neg_seed_3),
                self.derive(1, neg_seed_1, neg_seed_2, neg_seed_3),
                self.derive(2, neg_seed_1, neg_seed_2, neg_seed_3),
            ],
            opcode_key_table: self.opcode_key_table.clone(),
        }
    }
}
