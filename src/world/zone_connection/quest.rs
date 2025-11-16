//! Quests!

use crate::{
    constants::COMPLETED_LEVEQUEST_BITMASK_SIZE,
    ipc::zone::{ActiveQuest, QuestActiveList, ServerZoneIpcData, ServerZoneIpcSegment},
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

    pub async fn accept_quest(&mut self, id: u32) {
        let adjusted_id = id - 65536;

        // TODO: add to internal data model
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::AcceptQuest {
            quest_id: adjusted_id,
        });
        self.send_ipc_self(ipc).await;

        // Ensure its updated in the journal or whatever
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateQuest {
            index: 0,
            quest: ActiveQuest {
                id: adjusted_id as u16,
                sequence: 0xFF,
                ..Default::default()
            },
        });
        self.send_ipc_self(ipc).await;
    }

    pub async fn finish_quest(&mut self, id: u32) {
        let adjusted_id = id - 65536;

        // Ensure its updated in the journal or whatever
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateQuest {
            index: 0,
            quest: ActiveQuest::default(),
        });
        self.send_ipc_self(ipc).await;

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::FinishQuest {
            quest_id: adjusted_id as u16,
            flag1: 1,
            flag2: 1,
        });
        self.send_ipc_self(ipc).await;
    }
}
