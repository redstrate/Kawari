use diesel::prelude::*;

use super::{Friends, WorldDatabase, schema::friends::dsl::*};
use crate::GameData;
use kawari::ipc::zone::PlayerEntry;

impl WorldDatabase {
    fn get_friend_content_ids(&mut self, for_content_id: i64) -> Vec<i64> {
        friends
            .filter(content_id.eq(for_content_id))
            .select(friend_content_id)
            .load(&mut self.connection)
            .unwrap_or_default()
    }

    // TODO: Fix the HACK below.
    // HACK: For the moment we don't support this bitfield "unk2" in SocialList, and for the friend list to work correctly, we need to set a flag of 32 for the invite sender (waiting for friend approval) while the invitee needs 48 (waiting for approval + "accepted" (16) which makes no sense, since the invitee has yet to respond, but it's what the client wants).
    pub fn friend_request_pending_status(&mut self, for_content_id: i64) -> u8 {
        friends
            .filter(friend_content_id.eq(for_content_id))
            .select(is_pending)
            .first::<i32>(&mut self.connection)
            .unwrap_or_default() as u8
    }

    pub fn accept_friend(&mut self, for_content_id: i64, for_friend_content_id: i64) {
        // Update both ends of the invitation since they're mutual friends now.
        diesel::update(friends)
            .filter(content_id.eq(for_content_id))
            .filter(friend_content_id.eq(for_friend_content_id))
            .set(is_pending.eq(16)) // 16 indicates the friend invite sequence is over and the client should display them as a friend
            .execute(&mut self.connection)
            .unwrap();
        diesel::update(friends)
            .filter(content_id.eq(for_friend_content_id))
            .filter(friend_content_id.eq(for_content_id))
            .set(is_pending.eq(16)) // 16 indicates the friend invite sequence is over and the client should display them as a friend
            .execute(&mut self.connection)
            .unwrap();
    }

    // TODO: Fix this `pending` parameter, it's some sort of bitmask (search this file for HACK for further details)
    pub fn add_to_friend_list(&mut self, fwen_content_id: i64, my_content_id: i64, pending: i32) {
        if my_content_id == fwen_content_id {
            tracing::error!(
                "Player with content id {my_content_id} attempted to add themselves to their friend list. Ignoring request."
            );
            return;
        }
        let time = diesel::select(super::unixepoch())
            .get_result::<i64>(&mut self.connection)
            .unwrap();

        let friend = Friends {
            id: fastrand::i64(..),
            content_id: my_content_id,
            friend_content_id: fwen_content_id,
            group_icon: 0,
            invite_time: time,
            is_pending: pending,
        };

        diesel::insert_into(friends)
            .values(friend)
            .execute(&mut self.connection)
            .unwrap();
    }

    pub fn remove_from_friend_list(&mut self, their_content_id: i64, my_content_id: i64) {
        diesel::delete(
            friends
                .filter(content_id.eq(my_content_id))
                .filter(friend_content_id.eq(their_content_id)),
        )
        .execute(&mut self.connection)
        .unwrap();
        diesel::delete(
            friends
                .filter(content_id.eq(their_content_id))
                .filter(friend_content_id.eq(my_content_id)),
        )
        .execute(&mut self.connection)
        .unwrap();
    }

    pub fn find_friend_list(
        &mut self,
        game_data: &mut GameData,
        for_content_id: i64,
    ) -> Vec<PlayerEntry> {
        let mut friend_entries = Vec::<PlayerEntry>::new();

        let friend_content_ids = self.get_friend_content_ids(for_content_id);

        // If they have no friends, just return an empty list that the zone connection can reuse.
        if friend_content_ids.is_empty() {
            return vec![PlayerEntry::default(); 10];
        }

        for their_content_id in friend_content_ids {
            let mut friend_entry = self.get_player_entry(game_data, their_content_id);
            friend_entry.timestamp = self.get_friend_timestamp(for_content_id, their_content_id);
            friend_entries.push(friend_entry);
        }
        friend_entries
    }

    fn get_friend_timestamp(&mut self, for_content_id: i64, for_friend_content_id: i64) -> u32 {
        let time = friends
            .select(invite_time)
            .filter(content_id.eq(for_content_id))
            .filter(friend_content_id.eq(for_friend_content_id))
            .first::<i64>(&mut self.connection)
            .unwrap_or_default();
        time as u32
    }
}
