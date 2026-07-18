use async_trait::async_trait;
use bitflags::bitflags;
use kawari::{
    common::{CharacterMode, ObjectTypeId, value_to_flag_byte_index_value_quests},
    ipc::zone::{
        ActorControlCategory, Condition, LiveEventType, SceneFlags, ServerZoneIpcData,
        ServerZoneIpcSegment,
    },
};

use crate::{Event, EventHandler, ItemInfoQuery, ZoneConnection, inventory::Item, lua::LuaPlayer};

/// For gathering events.
#[derive(Debug)]
pub struct GatheringEventHandler {
    count: u8,
}

impl GatheringEventHandler {
    pub fn new(count: u8) -> Self {
        Self { count }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Default)]
pub struct GatheringItemFlag(u8);

bitflags! {
    impl GatheringItemFlag : u8 {
        const GATHERING_CHANCE_BONUS = 1;
        const BOON_CHANCE_BONUS = 2;
        const UNK1 = 4;
        const HIDDEN = 8;
        const RARE = 16;
        const BONUS = 32;
        const NOT_GATHERED_YET = 64;
        const UNK2 = 128;
    }
}

impl std::fmt::Debug for GatheringItemFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[async_trait]
impl EventHandler for GatheringEventHandler {
    async fn on_talk(&self, event: &Event, _target_id: ObjectTypeId, player: &mut LuaPlayer) {
        // TODO: figure out these params
        player.play_scene(
            0,
            SceneFlags::NO_DEFAULT_CAMERA,
            vec![
                0,
                event.id & 0xFFFF,
                2147485320,
                u32::from_le_bytes([self.count, 0, self.count, 0]), // first: count, second: ??, third: remaining count, fourth: ??
                24,
                1310820,
                67305316,
                9437184,
                108,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
                65636,
                67305316,
                1638400,
                29,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                32755,
                0,
                0,
                1121910784,
                32755,
                2616352782,
                1,
                1114399744,
                0,
                0,
                0,
                33024,
                0,
                0,
                0,
                0,
                0,
            ],
        );
    }

    async fn on_yield(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        scene: u16,
        _yield_id: u8,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        // TODO: store this on begin gather
        let items;
        {
            let mut gamedata = connection.gamedata.lock();
            items = gamedata.get_gathering_point_items(event.id & 0xFFFF);
        }

        if results[2] == 2 {
            // gather
            let item_index = results[1];
            let gather_item_id = items[item_index as usize].item_id;

            // plays the animation
            player.play_scene(1, SceneFlags::NO_DEFAULT_CAMERA, vec![2, 266]);

            // Add item to their inventory
            {
                let mut gamedata = connection.gamedata.lock();

                if let Some(item_info) =
                    gamedata.get_item_info(ItemInfoQuery::ById(gather_item_id as u32))
                {
                    connection
                        .player_data
                        .inventory
                        .add_in_next_free_slot(Item::new(&item_info, 1));
                }
            }

            connection.send_inventory().await;

            if !player
                .player_data
                .quest
                .gathered_gathering_items
                .contains(items[item_index as usize].gathering_id as u32)
            {
                let (value, index) = value_to_flag_byte_index_value_quests(
                    items[item_index as usize].gathering_id as u32,
                );

                connection.player_data.quest.gathered_gathering_items.data[index as usize] ^= value;

                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::RecordGatheringLog {
                    index: index as u8,
                    value: connection.player_data.quest.gathered_gathering_items.data
                        [index as usize],
                });
                connection.send_ipc_self(ipc).await;

                connection
                    .actor_control_self(ActorControlCategory::LiveEvent {
                        event: LiveEventType::RecordInGatheringLog {
                            item_id: gather_item_id as u32,
                        },
                    })
                    .await;

                // Add EXP
                // TODO: don't use placeholder EXP
                // TODO: the first time EXP doesn't take into account bonus?
                connection.add_exp(516).await;
            }

            // Add EXP
            // TODO: don't use placeholder EXP
            connection.add_exp(96).await;

            // The item was added to your inventory.
            connection
                .actor_control_self(ActorControlCategory::LogMessage {
                    log_message: 789,
                    id: gather_item_id as u32,
                })
                .await;

            return;
        }

        // quit
        if scene == 0 && results[2] == 0 {
            player.finish_event();
            return;
        }

        let mut params = vec![
            7,
            event.id & 0xFFFF,
            2147485320,
            u32::from_le_bytes([self.count, 0, self.count, 0]), // first: count, second: ??, third: remaining count, fourth: ??
        ];

        for item in items {
            let mut flags = GatheringItemFlag::default();
            if item.hidden {
                flags.insert(GatheringItemFlag::HIDDEN);
            }
            if !player
                .player_data
                .quest
                .gathered_gathering_items
                .contains(item.gathering_id as u32)
            {
                flags.insert(GatheringItemFlag::NOT_GATHERED_YET);
            }

            params.append(&mut vec![
                item.gathering_id as u32,
                u32::from_le_bytes([0, 0, item.level, 0]), // first: ??, second: ??, third: displayed level, fourth: ??
                u32::from_le_bytes([100, 0, 1, flags.0]), // first: gathering chance, second: HQ gathering chance, third: count, fourth: flag (see above)
                0,
                0,
                0,
            ]);
        }

        player.play_scene(0, SceneFlags::NO_DEFAULT_CAMERA, params);
    }

    fn condition(&self) -> Condition {
        Condition::ExecutingGatheringAction
    }

    fn character_mode(&self) -> CharacterMode {
        CharacterMode::Gathering
    }
}
