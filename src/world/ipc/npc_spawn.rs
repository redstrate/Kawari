use binrw::binrw;

use super::CommonSpawn;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct NpcSpawn {
    pub common: CommonSpawn,
    pub padding: [u8; 10],
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::{
        common::INVALID_OBJECT_ID,
        world::ipc::{CharacterMode, ObjectKind},
    };

    use super::*;

    #[test]
    fn read_carbuncle() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/npc_spawn.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let npc_spawn = NpcSpawn::read_le(&mut buffer).unwrap();
        assert_eq!(npc_spawn.common.hp_max, 1393);
        assert_eq!(npc_spawn.common.hp_curr, 1393);
        assert_eq!(npc_spawn.common.mp_curr, 10000);
        assert_eq!(npc_spawn.common.mp_max, 10000);
        assert_eq!(npc_spawn.common.display_flags, 0);
        assert_eq!(npc_spawn.common.pos.x, -64.17707);
        assert_eq!(npc_spawn.common.pos.y, -2.0206506);
        assert_eq!(npc_spawn.common.pos.z, 15.913875);
        assert_eq!(npc_spawn.common.model_chara, 411);
        assert_eq!(npc_spawn.common.bnpc_base, 13498);
        assert_eq!(npc_spawn.common.bnpc_name, 10261);
        assert_eq!(npc_spawn.common.spawn_index, 56);
        assert_eq!(npc_spawn.common.mode, CharacterMode::Normal);
        assert_eq!(npc_spawn.common.object_kind, ObjectKind::BattleNpc);
        assert_eq!(npc_spawn.common.subtype, 2);
        assert_eq!(npc_spawn.common.battalion, 0);
    }

    #[test]
    fn read_tiny_mandragora() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/tiny_mandragora.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let npc_spawn = NpcSpawn::read_le(&mut buffer).unwrap();
        assert_eq!(npc_spawn.common.hp_max, 91);
        assert_eq!(npc_spawn.common.hp_curr, 91);
        assert_eq!(npc_spawn.common.mp_curr, 0);
        assert_eq!(npc_spawn.common.mp_max, 0);
        assert_eq!(npc_spawn.common.display_flags, 0);
        assert_eq!(npc_spawn.common.pos.x, 116.99154);
        assert_eq!(npc_spawn.common.pos.y, 76.64936);
        assert_eq!(npc_spawn.common.pos.z, -187.02414);
        assert_eq!(npc_spawn.common.model_chara, 297);
        assert_eq!(npc_spawn.common.bnpc_base, 118);
        assert_eq!(npc_spawn.common.bnpc_name, 405);
        assert_eq!(npc_spawn.common.spawn_index, 14);
        assert_eq!(npc_spawn.common.mode, CharacterMode::Normal);
        assert_eq!(npc_spawn.common.object_kind, ObjectKind::BattleNpc);
        assert_eq!(npc_spawn.common.subtype, 5);
        assert_eq!(npc_spawn.common.battalion, 4);
        assert_eq!(npc_spawn.common.parent_actor_id, INVALID_OBJECT_ID);
        assert_eq!(npc_spawn.common.spawner_id, INVALID_OBJECT_ID);
    }
}
