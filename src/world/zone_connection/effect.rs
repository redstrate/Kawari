//! Status effect list handling.

use crate::{
    common::ObjectId,
    world::{ToServer, ZoneConnection},
};

impl ZoneConnection {
    pub async fn gain_effect(&mut self, effect_id: u16, effect_param: u16, effect_duration: f32) {
        // The server will update our state later
        self.handle
            .send(ToServer::GainEffect(
                self.id,
                self.player_data.actor_id,
                effect_id,
                effect_param,
                effect_duration,
                self.player_data.actor_id,
            ))
            .await;
    }

    pub async fn lose_effect(
        &mut self,
        effect_id: u16,
        effect_param: u16,
        effect_source_actor_id: ObjectId,
    ) {
        // The server will update our state later
        self.handle
            .send(ToServer::LoseEffect(
                self.id,
                self.player_data.actor_id,
                effect_id,
                effect_param,
                effect_source_actor_id,
            ))
            .await;
    }
}
