use binrw::binrw;

use super::CommonSpawn;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct PlayerSpawn {
    // also shows up in the friends list.
    pub some_unique_id: u32,

    #[brw(pad_before = 4)] // always empty?
    pub content_id: u64,

    pub common: CommonSpawn,

    pub padding: [u8; 2],
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::world::ipc::{CharacterMode, ObjectKind};

    use super::*;

    #[test]
    fn read_playerspawn() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/player_spawn.dat");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let player_spawn = PlayerSpawn::read_le(&mut buffer).unwrap();
        assert_eq!(player_spawn.common.current_world_id, 0x4F);
        assert_eq!(player_spawn.common.home_world_id, 0x4F);
        assert_eq!(player_spawn.common.hp_curr, 159);
        assert_eq!(player_spawn.common.hp_max, 159);
        assert_eq!(player_spawn.common.mp_curr, 10000);
        assert_eq!(player_spawn.common.mp_max, 10000);
        assert_eq!(player_spawn.common.mode, CharacterMode::Normal);
        assert_eq!(player_spawn.common.spawn_index, 0);
        assert_eq!(player_spawn.common.level, 1);
        assert_eq!(player_spawn.common.class_job, 1); // adventurer
        assert_eq!(player_spawn.common.scale, 36);
        assert_eq!(player_spawn.common.pos.x, 40.519722);
        assert_eq!(player_spawn.common.pos.y, 4.0);
        assert_eq!(player_spawn.common.pos.z, -150.33124);
        assert_eq!(player_spawn.common.name, "Lavenaa Warren");
        assert_eq!(player_spawn.common.look.race, 1);
        assert_eq!(player_spawn.common.look.gender, 1);
        assert_eq!(player_spawn.common.look.bust, 100);
        assert_eq!(player_spawn.common.fc_tag, "");
        assert_eq!(player_spawn.common.subtype, 4);
        assert_eq!(player_spawn.common.model_chara, 0);
        assert_eq!(player_spawn.common.object_kind, ObjectKind::Player);
        assert_eq!(player_spawn.common.display_flags, 262144);
    }
}
