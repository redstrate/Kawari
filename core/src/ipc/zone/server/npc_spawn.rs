use binrw::binrw;

use crate::ipc::zone::online_status::OnlineStatus;

use super::{CommonSpawn, GameMasterRank};

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct NpcSpawn {
    /// Refers to a Game Object ID in the zone, usually an LGB that this enemy is "attached" to.
    pub gimmick_id: u32,
    pub u1b: u8,
    pub u2b: u8,
    pub gm_rank: GameMasterRank, // FIXME: lol really? what does an NPC need GM rank privileges for?
    pub u3b: u8,

    pub aggression_mode: u8,
    pub online_status: OnlineStatus,
    pub u5a: u8,
    /// How many other BNpcs can be linked in this family.
    pub max_links: u8,
    /// If not zero, specifies which family this BNpc is linked to.
    pub link_family: u8,
    /// How far the link family can be apart.
    pub link_range: u8,
    pub u5d: u8,
    pub u5e: u8,

    pub common: CommonSpawn,
    pub padding: [u8; 14],
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

        let npc_spawn = NpcSpawn::read_le(&mut buffer).unwrap();
        assert_eq!(npc_spawn.common.max_hp, 973);
        assert_eq!(npc_spawn.common.hp, 973);
        assert_eq!(npc_spawn.common.mp, 10000);
        assert_eq!(npc_spawn.common.max_mp, 10000);
        //assert_eq!(npc_spawn.common.display_flags, DisplayFlag::NONE);
        assert_eq!(npc_spawn.common.position.x, 4.883462);
        assert_eq!(npc_spawn.common.position.y, 40.04264);
        assert_eq!(npc_spawn.common.position.z, 11.821917);
        assert_eq!(npc_spawn.common.model_chara, 411);
        assert_eq!(npc_spawn.common.npc_base, 13498);
        assert_eq!(npc_spawn.common.npc_name, 10261);
        assert_eq!(npc_spawn.common.spawn_index, 12);
        assert_eq!(npc_spawn.common.mode, CharacterMode::Normal);
        assert_eq!(npc_spawn.common.mode_arg, 0);
        assert_eq!(
            npc_spawn.common.object_kind,
            ObjectKind::BattleNpc(BattleNpcSubKind::Pet)
        );
        assert_eq!(npc_spawn.common.battalion, 0);
        assert_eq!(npc_spawn.aggression_mode, 1); // passive
        assert!(!npc_spawn.common.tether_id.is_valid());
        assert_eq!(npc_spawn.common.handler_id, HandlerId(0));
        assert_eq!(npc_spawn.common.layout_id, 0);
        assert_eq!(npc_spawn.online_status, OnlineStatus::Offline);
        assert_eq!(npc_spawn.common.name, "カーバンクル");
    }

    #[test]
    fn read_tiny_mandragora() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("tiny_mandragora.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let npc_spawn = NpcSpawn::read_le(&mut buffer).unwrap();
        assert_eq!(npc_spawn.common.max_hp, 91);
        assert_eq!(npc_spawn.common.hp, 91);
        assert_eq!(npc_spawn.common.mp, 0);
        assert_eq!(npc_spawn.common.max_mp, 0);
        assert_eq!(npc_spawn.common.display_flags, DisplayFlag::NONE);
        assert_eq!(npc_spawn.common.position.x, 61.169727);
        assert_eq!(npc_spawn.common.position.y, 64.56608);
        assert_eq!(npc_spawn.common.position.z, -168.08115);
        assert_eq!(npc_spawn.common.model_chara, 297);
        assert_eq!(npc_spawn.common.npc_base, 118);
        assert_eq!(npc_spawn.common.npc_name, 405);
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
        assert!(!npc_spawn.common.tether_id.is_valid());
        assert_eq!(npc_spawn.common.layout_id, 3929856);
        assert_eq!(npc_spawn.aggression_mode, 1); // passive
        assert_eq!(npc_spawn.online_status, OnlineStatus::Offline);
        assert_eq!(npc_spawn.common.name, "タイニー・マンドラゴ");
    }
}
