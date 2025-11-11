//! Quests!

use crate::{
    constants::COMPLETED_LEVEQUEST_BITMASK_SIZE,
    ipc::zone::{QuestActiveList, ServerZoneIpcData, ServerZoneIpcSegment},
    world::ZoneConnection,
};

impl ZoneConnection {
    pub async fn send_quest_information(&mut self) {
        // quest active list
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::QuestActiveList(
                QuestActiveList::default(),
            ));
            self.send_ipc_self(ipc).await;
        }

        // quest complete list
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::QuestCompleteList {
                completed_quests: self.player_data.unlocks.completed_quests.0.clone(),
                unk2: vec![0xFF; 65],
            });
            self.send_ipc_self(ipc).await;
        }

        // levequest complete list
        // NOTE: all levequests are unlocked by default
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LevequestCompleteList {
                completed_levequests: vec![0xFF; COMPLETED_LEVEQUEST_BITMASK_SIZE],
                unk2: Vec::default(),
            });
            self.send_ipc_self(ipc).await;
        }
    }
}
