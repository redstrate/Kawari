use binrw::binrw;

use crate::ipc::zone::online_status::OnlineStatus;

use super::{CommonSpawn, GameMasterRank};

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct SpawnPlayer {
    /// The account ID of the player.
    pub account_id: u64,
    /// The content ID of the player.
    pub content_id: u64,

    /// Index into the Title Excel sheet.
    pub title_id: u16,
    pub timeline_base_override: u16,
    /// The World this player is currently spawned in.
    pub current_world_id: u16,
    /// The World this player originates from.
    pub home_world_id: u16,

    /// What GM rank this player has.
    pub gm_rank: GameMasterRank,
    pub u3c: u8,
    pub u4: u8,
    /// The current online status of this player.
    pub online_status: OnlineStatus,

    /// The pose this character is in.
    pub pose: u8,
    pub u5a: u16,
    pub u5b: u8,

    /// Other spawn data such as appearance and equipped items.
    pub common: CommonSpawn,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::common::CharacterMode;
    use crate::ipc::zone::{DisplayFlag, ObjectKind, PlayerSubKind};

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_playerspawn() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("player_spawn.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let player_spawn = SpawnPlayer::read_le(&mut buffer).unwrap();
        assert_eq!(player_spawn.current_world_id, 0x4F);
        assert_eq!(player_spawn.home_world_id, 0x4F);
        assert_eq!(player_spawn.common.health_points, 159);
        assert_eq!(player_spawn.common.max_health_points, 159);
        assert_eq!(player_spawn.common.resource_points, 10000);
        assert_eq!(player_spawn.common.max_resource_points, 10000);
        assert_eq!(player_spawn.common.mode, CharacterMode::Normal);
        assert_eq!(player_spawn.common.mode_arg, 0);
        assert_eq!(player_spawn.common.spawn_index, 0);
        assert_eq!(player_spawn.common.level, 1);
        assert_eq!(player_spawn.common.class_job, 1); // adventurer
        assert_eq!(player_spawn.common.position.0.x, 40.360653);
        assert_eq!(player_spawn.common.position.0.y, 4.0);
        assert_eq!(player_spawn.common.position.0.z, -152.85175);
        assert_eq!(player_spawn.common.name, "Lavenaa Warren");
        assert_eq!(player_spawn.common.look.race, 1);
        assert_eq!(player_spawn.common.look.gender, 1);
        assert_eq!(player_spawn.common.look.bust, 100);
        assert_eq!(player_spawn.common.fc_tag, "");
        assert_eq!(player_spawn.common.model_chara, 0);
        assert_eq!(
            player_spawn.common.object_kind,
            ObjectKind::Player(PlayerSubKind::Player)
        );
        assert_eq!(player_spawn.common.display_flags, DisplayFlag::UNK1);
        assert_eq!(player_spawn.online_status, OnlineStatus::Offline);
    }
}
