use binrw::binrw;

use crate::common::{LegacyEquipmentModelId, WeaponModelId};

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct Equip {
    pub main_weapon_id: WeaponModelId,
    pub sub_weapon_id: WeaponModelId,
    /// For the Free Company crest.
    pub crest_bitfield: u8,
    /// Current class of the player. Index into the ClassJob Excel sheet.
    pub classjob_id: u8,
    /// Level of the current class.
    pub level: u8,
    pub unk1: u8,
    /// Equipment model IDs.
    pub models: [LegacyEquipmentModelId; 10],
    #[brw(pad_after = 2)] // padding
    /// Second dye stains for the given `models`.
    pub second_model_stain_ids: [u8; 10],
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_containerinfo() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("equip.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let equip = Equip::read_le(&mut buffer).unwrap();
        assert_eq!(
            equip.main_weapon_id,
            WeaponModelId {
                id: 201,
                model_type: 43,
                variant: 1,
                stains: [0, 0]
            }
        );
        assert_eq!(equip.sub_weapon_id, WeaponModelId::default());
        assert_eq!(equip.crest_bitfield, 0);
        assert_eq!(equip.classjob_id, 1);
        assert_eq!(equip.level, 1);
        assert_eq!(equip.unk1, 0);
        assert_eq!(
            equip.models,
            [
                LegacyEquipmentModelId {
                    id: 0,
                    variant: 0,
                    stain: 0
                },
                LegacyEquipmentModelId {
                    id: 0,
                    variant: 0,
                    stain: 0
                },
                LegacyEquipmentModelId {
                    id: 84,
                    variant: 2,
                    stain: 0
                },
                LegacyEquipmentModelId {
                    id: 84,
                    variant: 2,
                    stain: 0
                },
                LegacyEquipmentModelId {
                    id: 84,
                    variant: 2,
                    stain: 0
                },
                LegacyEquipmentModelId {
                    id: 1,
                    variant: 2,
                    stain: 0
                },
                LegacyEquipmentModelId {
                    id: 1,
                    variant: 2,
                    stain: 0
                },
                LegacyEquipmentModelId {
                    id: 1,
                    variant: 2,
                    stain: 0
                },
                LegacyEquipmentModelId {
                    id: 0,
                    variant: 0,
                    stain: 0
                },
                LegacyEquipmentModelId {
                    id: 1,
                    variant: 2,
                    stain: 0
                }
            ]
        );
        assert_eq!(equip.second_model_stain_ids, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }
}
