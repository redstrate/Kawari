use binrw::binrw;

use super::{CommonSpawn, GameMasterRank, OnlineStatus};

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct NpcSpawn {
    pub gimmick_id: u32,
    pub u1b: u8,
    pub u2b: u8,
    pub gm_rank: GameMasterRank, // lol really? what does an NPC need GM rank privileges for?
    pub u3b: u8,

    pub aggression_mode: u8,
    pub online_status: OnlineStatus,
    pub u5a: u8,
    pub pose: u8,

    pub u5b: u32,

    pub common: CommonSpawn,
    pub padding: [u8; 14],
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::{
        common::INVALID_OBJECT_ID,
        ipc::zone::{BattleNpcSubKind, CharacterMode, DisplayFlag, ObjectKind, OnlineStatus},
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
        assert_eq!(npc_spawn.common.hp_max, 973);
        assert_eq!(npc_spawn.common.hp_curr, 973);
        assert_eq!(npc_spawn.common.mp_curr, 10000);
        assert_eq!(npc_spawn.common.mp_max, 10000);
        //assert_eq!(npc_spawn.common.display_flags, DisplayFlag::NONE);
        assert_eq!(npc_spawn.common.pos.x, 4.883462);
        assert_eq!(npc_spawn.common.pos.y, 40.04264);
        assert_eq!(npc_spawn.common.pos.z, 11.821917);
        assert_eq!(npc_spawn.common.model_chara, 411);
        assert_eq!(npc_spawn.common.bnpc_base, 13498);
        assert_eq!(npc_spawn.common.bnpc_name, 10261);
        assert_eq!(npc_spawn.common.spawn_index, 12);
        assert_eq!(npc_spawn.common.mode, CharacterMode::Normal);
        assert_eq!(
            npc_spawn.common.object_kind,
            ObjectKind::BattleNpc(BattleNpcSubKind::Pet)
        );
        assert_eq!(npc_spawn.common.battalion, 0);
        assert_eq!(npc_spawn.aggression_mode, 1); // passive
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
        assert_eq!(npc_spawn.common.hp_max, 91);
        assert_eq!(npc_spawn.common.hp_curr, 91);
        assert_eq!(npc_spawn.common.mp_curr, 0);
        assert_eq!(npc_spawn.common.mp_max, 0);
        assert_eq!(npc_spawn.common.display_flags, DisplayFlag::NONE);
        assert_eq!(npc_spawn.common.pos.x, 61.169727);
        assert_eq!(npc_spawn.common.pos.y, 64.56608);
        assert_eq!(npc_spawn.common.pos.z, -168.08115);
        assert_eq!(npc_spawn.common.model_chara, 297);
        assert_eq!(npc_spawn.common.bnpc_base, 118);
        assert_eq!(npc_spawn.common.bnpc_name, 405);
        assert_eq!(npc_spawn.common.spawn_index, 18);
        assert_eq!(npc_spawn.common.mode, CharacterMode::Normal);
        assert_eq!(
            npc_spawn.common.object_kind,
            ObjectKind::BattleNpc(BattleNpcSubKind::Enemy)
        );
        assert_eq!(npc_spawn.common.battalion, 4);
        assert_eq!(npc_spawn.common.parent_actor_id, INVALID_OBJECT_ID);
        assert_eq!(npc_spawn.common.spawner_id, INVALID_OBJECT_ID);
        assert_eq!(npc_spawn.aggression_mode, 1); // passive
        assert_eq!(npc_spawn.online_status, OnlineStatus::Offline);
        assert_eq!(npc_spawn.common.name, "タイニー・マンドラゴ");
    }
}
