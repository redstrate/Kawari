use binrw::binrw;
use bitflags::bitflags;

use crate::common::{read_bool_from, write_bool_as};

use super::CommonSpawn;

#[binrw]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CharacterDataFlag(u8);

bitflags! {
    impl CharacterDataFlag: u8 {
        const NONE = 0x0;
        /// If set, this marks the enemy as "hostile" including changing the nameplate icon.
        const HOSTILE = 0x2;
    }
}

impl Default for CharacterDataFlag {
    fn default() -> Self {
        Self::NONE
    }
}

impl std::fmt::Debug for CharacterDataFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct SpawnNpc {
    /// Refers to a Game Object ID in the zone, usually an LGB that this enemy is "attached" to.
    pub gimmick_id: u32,
    /// At least filled for Quests, where this is the originating Event NPC layout ID if it turned into a Battle NPC.
    pub enpc_id: u32,

    pub character_data_flags: CharacterDataFlag,
    /// Roughly correlates to mob difficulty, supposedly:
    /// 0 = grey alien icon
    /// 1 = spiky blue icon of some sort
    /// 2 = blue spiky but more triangular, like it has horns
    /// 3 = squircle with three dots inside it
    /// 4 = same squircle but with thin, tiny horns
    /// 5 = same as 4
    /// 6 = triangle but only with two circles and what looks like closed eyes
    /// 7 = super big horns
    pub character_data_icon: u8,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub unk_a: bool,
    /// How many other BNpcs can be linked in this family.
    pub max_links: u8,
    /// If not zero, specifies which family this BNpc is linked to.
    pub link_family: u8,
    /// How far the link family can be apart.
    pub link_range: u8,
    pub u5d: u8,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub unk_f: bool,

    /// Other spawn data such as appearance and equipped items.
    pub common: CommonSpawn,
    // The following fields modify stuff in ModelContainer.
    pub unk288: u8,
    pub unk289: u8,
    pub unk28a: u8,
    pub unk28b: u8,
    pub unk28c: u8,
    #[brw(pad_after = 2)] // i think is empty, not read by the client
    pub unk28d: u8,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::{
        common::{CharacterMode, HandlerId},
        ipc::zone::{BattleNpcSubKind, DisplayFlag, ObjectKind},
        server_zone_tests_dir,
    };

    use super::*;

    #[test]
    fn read_carbuncle() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("npc_spawn.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let npc_spawn = SpawnNpc::read_le(&mut buffer).unwrap();
        assert_eq!(npc_spawn.common.max_health_points, 973);
        assert_eq!(npc_spawn.common.health_points, 973);
        assert_eq!(npc_spawn.common.resource_points, 10000);
        assert_eq!(npc_spawn.common.max_resource_points, 10000);
        //assert_eq!(npc_spawn.common.display_flags, DisplayFlag::NONE);
        assert_eq!(npc_spawn.common.position.x, 4.883462);
        assert_eq!(npc_spawn.common.position.y, 40.04264);
        assert_eq!(npc_spawn.common.position.z, 11.821917);
        assert_eq!(npc_spawn.common.model_chara, 411);
        assert_eq!(npc_spawn.common.base_id, 13498);
        assert_eq!(npc_spawn.common.name_id, 10261);
        assert_eq!(npc_spawn.common.spawn_index, 12);
        assert_eq!(npc_spawn.common.mode, CharacterMode::Normal);
        assert_eq!(npc_spawn.common.mode_arg, 0);
        assert_eq!(
            npc_spawn.common.object_kind,
            ObjectKind::BattleNpc(BattleNpcSubKind::Pet)
        );
        assert_eq!(npc_spawn.common.battalion, 0);
        assert_eq!(
            npc_spawn.character_data_flags,
            CharacterDataFlag::from_bits_retain(0x1)
        );
        assert!(!npc_spawn.common.tether_target_id.is_valid());
        assert_eq!(npc_spawn.common.handler_id, HandlerId(0));
        assert_eq!(npc_spawn.common.layout_id, 0);
        assert_eq!(npc_spawn.character_data_icon, 0);
        assert_eq!(npc_spawn.common.name, "カーバンクル");
    }

    #[test]
    fn read_tiny_mandragora() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("tiny_mandragora.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let npc_spawn = SpawnNpc::read_le(&mut buffer).unwrap();
        assert_eq!(npc_spawn.common.max_health_points, 91);
        assert_eq!(npc_spawn.common.health_points, 91);
        assert_eq!(npc_spawn.common.resource_points, 0);
        assert_eq!(npc_spawn.common.max_resource_points, 0);
        assert_eq!(npc_spawn.common.display_flags, DisplayFlag::NONE);
        assert_eq!(npc_spawn.common.position.x, 61.169727);
        assert_eq!(npc_spawn.common.position.y, 64.56608);
        assert_eq!(npc_spawn.common.position.z, -168.08115);
        assert_eq!(npc_spawn.common.model_chara, 297);
        assert_eq!(npc_spawn.common.base_id, 118);
        assert_eq!(npc_spawn.common.name_id, 405);
        assert_eq!(npc_spawn.common.spawn_index, 18);
        assert_eq!(npc_spawn.common.mode, CharacterMode::Normal);
        assert_eq!(npc_spawn.common.mode_arg, 0);
        assert_eq!(
            npc_spawn.common.object_kind,
            ObjectKind::BattleNpc(BattleNpcSubKind::Enemy)
        );
        assert_eq!(npc_spawn.common.battalion, 4);
        assert!(!npc_spawn.common.owner_id.is_valid());
        assert_eq!(npc_spawn.common.handler_id, HandlerId(0));
        assert!(!npc_spawn.common.tether_target_id.is_valid());
        assert_eq!(npc_spawn.common.layout_id, 3929856);
        assert_eq!(
            npc_spawn.character_data_flags,
            CharacterDataFlag::from_bits_retain(0x1)
        );
        assert_eq!(npc_spawn.character_data_icon, 0);
        assert_eq!(npc_spawn.common.name, "タイニー・マンドラゴ");
    }
}
