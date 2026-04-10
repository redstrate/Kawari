// ! The friend system.
use crate::{ToServer, ZoneConnection};
use kawari::{
    common::ObjectId,
    ipc::zone::{ServerZoneIpcData, ServerZoneIpcSegment},
};
impl ZoneConnection {
    pub async fn refresh_friend_list(&mut self) {
        // Only refresh if we ran out of results from a prior run.
        if self.friend_results.is_empty() {
            let mut db = self.database.lock();
            let mut game_data = self.gamedata.lock();
            self.friend_results =
                db.find_friend_list(&mut game_data, self.player_data.character.content_id);

            self.friend_index = 0;
        }
    }

    pub fn add_to_friend_list(&mut self, friend_content_id: u64, pending: i32) {
        let mut db = self.database.lock();
        db.add_to_friend_list(
            friend_content_id as i64,
            self.player_data.character.content_id,
            pending,
        );
    }

    pub async fn remove_from_friend_list(&mut self, their_content_id: u64, their_name: String) {
        let their_actor_id;
        {
            let mut db = self.database.lock();
            their_actor_id = db.find_actor_id(their_content_id);

            // If we can't find them for some reason, don't proceed.
            if their_actor_id == ObjectId::default() {
                tracing::warn!(
                    "Unable to find {}'s actor id (it was {:#?})! What happened?)",
                    their_content_id,
                    ObjectId::default()
                );
                return;
            }

            // NOTE: This removes each other on both sides, so the receiver doesn't need to do this
            db.remove_from_friend_list(
                their_content_id as i64,
                self.player_data.character.content_id,
            );
        }

        self.handle
            .send(ToServer::FriendRemoved(
                self.player_data.character.actor_id,
                self.player_data.character.content_id as u64,
                self.player_data.character.name.clone(),
                their_actor_id,
                their_content_id,
                their_name,
            ))
            .await;
    }

    pub async fn friend_removed(&mut self, their_content_id: u64, their_name: String) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::FriendRemoved {
            content_id: their_content_id,
            name: their_name.clone(),
            unk1: 1,
        });

        self.send_ipc_self(ipc).await;
    }
}
