//! Quests!

use crate::{ZoneConnection, inventory::Storage, zone_connection::PersistentQuest};
use kawari::{
    common::adjust_quest_id,
    constants::COMPLETED_LEVEQUEST_BITMASK_SIZE,
    ipc::zone::{
        ActiveQuest, QuestActiveList, QuestTracker, ServerZoneIpcData, ServerZoneIpcSegment,
        TrackedQuest,
    },
};

impl ZoneConnection {
    pub async fn send_quest_information(&mut self) {
        // quest active list
        {
            let mut quests = Vec::new();
            for quest in &self.player_data.active_quests {
                quests.push(ActiveQuest {
                    id: quest.id,
                    sequence: quest.sequence,
                    flags: 1,
                    ..Default::default()
                });
            }

            let ipc =
                ServerZoneIpcSegment::new(ServerZoneIpcData::QuestActiveList(QuestActiveList {
                    quests,
                }));
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

        // legacy quest complete list
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LegacyQuestList {
                bitmask: [0xFF; 40],
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

        // scenario guide
        {
            // TODO: temporary
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ScenarioGuide {
                quest_id_1: 39,
                next_quest_id: 85,
                layout_id: 1985113,
            });
            self.send_ipc_self(ipc).await;
        }

        self.send_quest_tracker().await;
    }

    pub async fn accept_quest(&mut self, id: u32) {
        let adjusted_id = adjust_quest_id(id);

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
                flags: 1,
                ..Default::default()
            },
        });
        self.send_ipc_self(ipc).await;

        // Then add it to our own internal data model
        self.player_data.active_quests.push(PersistentQuest {
            id: adjusted_id as u16,
            sequence: 0xFF,
        });

        self.send_quest_tracker().await;
    }

    pub async fn finish_quest(&mut self, id: u32) {
        let adjusted_id = adjust_quest_id(65536);

        // Remove it from our internal data model
        if let Some(index) = self
            .player_data
            .active_quests
            .iter()
            .position(|x| x.id == adjusted_id as u16)
        {
            self.player_data.active_quests.remove(index);
        }

        // Grant rewards
        let rewards;
        {
            let mut gamedata = self.gamedata.lock();
            rewards = gamedata.get_quest_rewards(id);
        }

        // Add gil
        // TODO: send log message
        self.player_data.inventory.currency.get_slot_mut(0).quantity += rewards.1;
        self.send_inventory().await;

        // Add exp
        self.add_exp(rewards.0 as i32).await;

        // Ensure its updated in the journal or whatever
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateQuest {
            index: 0,
            quest: ActiveQuest::default(),
        });
        self.send_ipc_self(ipc).await;

        self.player_data.unlocks.completed_quests.set(adjusted_id);

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::FinishQuest {
            quest_id: adjusted_id as u16,
            flag1: 1,
            flag2: 1,
        });
        self.send_ipc_self(ipc).await;

        self.send_quest_tracker().await;
    }

    pub async fn send_quest_tracker(&mut self) {
        // Right now we don't support tracking, so just send the first five quests.
        let mut tracked_quests = [TrackedQuest::default(); 5];
        for (i, _) in self.player_data.active_quests.iter().take(5).enumerate() {
            tracked_quests[i] = TrackedQuest {
                active: true,
                quest_index: i as u8,
            };
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::QuestTracker(QuestTracker {
            tracked_quests,
        }));
        self.send_ipc_self(ipc).await;
    }
}
