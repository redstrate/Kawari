//! Translates tasks and handles other information from `LuaPlayer`.

use crate::{
    ERR_INVENTORY_ADD_FAILED,
    common::{InstanceContentType, ItemInfoQuery, ObjectId, ObjectTypeId, ObjectTypeKind},
    constants::{
        ADVENTURE_BITMASK_SIZE, AETHER_CURRENT_BITMASK_SIZE,
        AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE, BUDDY_EQUIP_BITMASK_SIZE,
        CAUGHT_FISH_BITMASK_SIZE, CAUGHT_SPEARFISH_BITMASK_SIZE, CHOCOBO_TAXI_STANDS_BITMASK_SIZE,
        COMPLETED_QUEST_BITMASK_SIZE, CUTSCENE_SEEN_BITMASK_SIZE, GLASSES_STYLES_BITMASK_SIZE,
        MINION_BITMASK_SIZE, ORNAMENT_BITMASK_SIZE, TRIPLE_TRIAD_CARDS_BITMASK_SIZE,
    },
    inventory::{Item, Storage},
    ipc::zone::{ActorControlCategory, ActorControlSelf, EventType},
    world::{
        ToServer, ZoneConnection,
        lua::{LuaPlayer, Task, load_init_script},
    },
};

impl ZoneConnection {
    pub async fn process_lua_player(&mut self, player: &mut LuaPlayer) {
        // First, send zone-related segments
        for segment in &player.zone_data.queued_segments {
            let mut edited_segment = segment.clone();
            edited_segment.target_actor = player.player_data.actor_id;
            self.send_segment(edited_segment).await;
        }
        player.zone_data.queued_segments.clear();

        // These are to run functions that could possibly generate more tasks.
        // We can't do this in the loop!'
        let mut run_enter_territory = false;
        let mut run_finish_event = false;

        let tasks = player.queued_tasks.clone();
        for task in &tasks {
            match task {
                Task::ChangeTerritory { zone_id } => self.change_zone(*zone_id).await,
                Task::SetRemakeMode(remake_mode) => self
                    .database
                    .set_remake_mode(player.player_data.content_id, *remake_mode),
                Task::Warp { warp_id } => {
                    self.warp(*warp_id).await;
                }
                Task::BeginLogOut => self.begin_log_out().await,
                Task::FinishEvent {
                    handler_id,
                    finish_type,
                } => {
                    self.event_finish(*handler_id, *finish_type).await;
                    run_finish_event = true;
                }
                Task::SetClassJob { classjob_id } => {
                    self.player_data.classjob_id = *classjob_id;
                    self.update_class_info().await;
                }
                Task::WarpAetheryte { aetheryte_id } => {
                    self.warp_aetheryte(*aetheryte_id).await;
                }
                Task::ReloadScripts => {
                    self.reload_scripts();
                }
                Task::ToggleInvisibility { invisible } => {
                    self.toggle_invisibility(*invisible).await;
                }
                Task::Unlock { id } => {
                    self.player_data.unlocks.unlocks.set(*id);

                    self.actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::ToggleUnlock {
                            id: *id,
                            unlocked: true,
                        },
                    })
                    .await;
                }
                Task::UnlockAetheryte { id, on } => {
                    let unlock_all = *id == 0;
                    if unlock_all {
                        for i in 1..239 {
                            if *on {
                                self.player_data.unlocks.aetherytes.set(i);
                            } else {
                                self.player_data.unlocks.aetherytes.clear(i);
                            }

                            self.actor_control_self(ActorControlSelf {
                                category: ActorControlCategory::LearnTeleport {
                                    id: i,
                                    unlocked: *on,
                                },
                            })
                            .await;
                        }
                    } else {
                        if *on {
                            self.player_data.unlocks.aetherytes.set(*id);
                        } else {
                            self.player_data.unlocks.aetherytes.clear(*id);
                        }

                        self.actor_control_self(ActorControlSelf {
                            category: ActorControlCategory::LearnTeleport {
                                id: *id,
                                unlocked: *on,
                            },
                        })
                        .await;
                    }
                }
                Task::SetLevel { level } => {
                    self.set_current_level(*level);
                    self.update_class_info().await;
                }
                Task::ChangeWeather { id } => {
                    self.change_weather(*id).await;
                }
                Task::AddGil { amount } => {
                    self.player_data.inventory.currency.get_slot_mut(0).quantity += *amount;
                    self.send_inventory(false).await;
                }
                Task::RemoveGil {
                    amount,
                    send_client_update,
                } => {
                    self.player_data.inventory.currency.get_slot_mut(0).quantity -= *amount;
                    if *send_client_update {
                        self.send_inventory(false).await;
                    }
                }
                Task::GmSetOrchestrion { value, id } => {
                    self.gm_set_orchestrion(*value, *id);
                }
                Task::AddItem {
                    id,
                    quantity,
                    send_client_update,
                } => {
                    let item_info;
                    {
                        let mut game_data = self.gamedata.lock();
                        item_info = game_data.get_item_info(ItemInfoQuery::ById(*id));
                    }
                    if let Some(item_info) = item_info {
                        if self
                            .player_data
                            .inventory
                            .add_in_next_free_slot(Item::new(item_info, *quantity))
                            .is_some()
                        {
                            if *send_client_update {
                                self.send_inventory(false).await;
                            }
                        } else {
                            tracing::error!(ERR_INVENTORY_ADD_FAILED);
                            self.send_notice(ERR_INVENTORY_ADD_FAILED).await;
                        }
                    } else {
                        tracing::error!(ERR_INVENTORY_ADD_FAILED);
                        self.send_notice(ERR_INVENTORY_ADD_FAILED).await;
                    }
                }
                Task::CompleteAllQuests {} => {
                    self.player_data.unlocks.completed_quests.0 =
                        vec![0xFF; COMPLETED_QUEST_BITMASK_SIZE];
                    self.send_quest_information().await;
                }
                Task::UnlockContent { id } => {
                    {
                        let mut game_data = self.gamedata.lock();
                        if let Some(instance_content_type) = game_data.find_type_for_content(*id) {
                            // Each id has to be subtracted by it's offset in the InstanceContent Excel sheet. For example, all guildheists start at ID 10000.
                            match instance_content_type {
                                InstanceContentType::Dungeon => {
                                    self.player_data
                                        .unlocks
                                        .unlocked_dungeons
                                        .set(*id as u32 - 1);
                                }
                                InstanceContentType::Raid => {
                                    self.player_data
                                        .unlocks
                                        .unlocked_raids
                                        .set(*id as u32 - 30001);
                                }
                                InstanceContentType::Guildhests => {
                                    self.player_data
                                        .unlocks
                                        .unlocked_guildhests
                                        .set(*id as u32 - 10001);
                                }
                                InstanceContentType::Trial => {
                                    self.player_data
                                        .unlocks
                                        .unlocked_trials
                                        .set(*id as u32 - 20001);
                                }
                                _ => {
                                    tracing::warn!(
                                        "Not sure what to do about {instance_content_type:?} {id}!"
                                    );
                                }
                            };
                        } else {
                            tracing::warn!("Unknown content {id}!");
                        }
                    }

                    self.actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::UnlockInstanceContent {
                            id: *id as u32,
                            unlocked: true,
                        },
                    })
                    .await;
                }
                Task::UpdateBuyBackList { list } => {
                    self.player_data.buyback_list = list.clone();
                }
                Task::AddExp { amount } => {
                    let current_exp;
                    {
                        let game_data = self.gamedata.lock();
                        current_exp = self.current_exp(&game_data);
                    }
                    self.set_current_exp(current_exp + amount);
                    self.update_class_info().await;
                }
                Task::StartEvent {
                    actor_id,
                    event_id,
                    event_type,
                    event_arg,
                } => {
                    self.start_event(*actor_id, *event_id, *event_type, *event_arg, player)
                        .await;
                    if *event_type == EventType::EnterTerritory {
                        run_enter_territory = true;
                    }
                }
                Task::SetInnWakeup { watched } => {
                    self.player_data.saw_inn_wakeup = *watched;
                }
                Task::ToggleMount { id } => {
                    let order;
                    {
                        let mut game_data = self.gamedata.lock();
                        order = game_data.find_mount_order(*id).unwrap_or(0);
                    }

                    let should_unlock = self.player_data.unlocks.mounts.toggle(*id);

                    self.actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::ToggleMountUnlock {
                            order: order as u32,
                            id: *id,
                            unlocked: should_unlock,
                        },
                    })
                    .await;
                }
                Task::MoveToPopRange { id } => {
                    self.handle
                        .send(ToServer::MoveToPopRange(
                            self.id,
                            self.player_data.actor_id,
                            *id,
                        ))
                        .await;
                }
                Task::SetHP { hp } => {
                    self.player_data.curr_hp = *hp;
                    self.update_hp_mp(
                        ObjectId(self.player_data.actor_id),
                        self.player_data.curr_hp,
                        self.player_data.curr_mp,
                    )
                    .await;
                }
                Task::SetMP { mp } => {
                    self.player_data.curr_mp = *mp;
                    self.update_hp_mp(
                        ObjectId(self.player_data.actor_id),
                        self.player_data.curr_hp,
                        self.player_data.curr_mp,
                    )
                    .await;
                }
                Task::ToggleGlassesStyle { id } => {
                    self.toggle_glasses_style(*id).await;
                }
                Task::ToggleGlassesStyleAll {} => {
                    let max_glasses_style_id = GLASSES_STYLES_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_glasses_style_id {
                        self.toggle_glasses_style(i).await;
                    }
                }
                Task::ToggleOrnament { id } => {
                    self.toggle_ornament(*id).await;
                }
                Task::ToggleOrnamentAll {} => {
                    let max_ornament_id = ORNAMENT_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_ornament_id {
                        self.toggle_ornament(i).await;
                    }
                }
                Task::UnlockBuddyEquip { id } => {
                    self.unlock_buddy_equip(*id).await;
                }
                Task::UnlockBuddyEquipAll {} => {
                    let max_buddy_equip_id = BUDDY_EQUIP_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_buddy_equip_id {
                        self.unlock_buddy_equip(i).await;
                    }
                }
                Task::ToggleChocoboTaxiStand { id } => {
                    self.toggle_chocobo_taxi_stand(*id).await;
                }
                Task::ToggleChocoboTaxiStandAll {} => {
                    let max_chocobo_taxi_stand_id = CHOCOBO_TAXI_STANDS_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_chocobo_taxi_stand_id {
                        self.toggle_chocobo_taxi_stand(i).await;
                    }
                }
                Task::ToggleCaughtFish { id } => {
                    self.toggle_caught_fish(*id).await;
                }
                Task::ToggleCaughtFishAll {} => {
                    let max_caught_fish_id = CAUGHT_FISH_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_caught_fish_id {
                        self.toggle_caught_fish(i).await;
                    }
                }
                Task::ToggleCaughtSpearfish { id } => {
                    self.toggle_caught_spearfish(*id).await;
                }
                Task::ToggleCaughtSpearfishAll {} => {
                    let max_caught_spearfish_id = CAUGHT_SPEARFISH_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_caught_spearfish_id {
                        self.toggle_caught_spearfish(i).await;
                    }
                }
                Task::ToggleTripleTriadCard { id } => {
                    self.toggle_triple_triad_card(*id).await;
                }
                Task::ToggleTripleTriadCardAll {} => {
                    let max_triple_triad_card_id = TRIPLE_TRIAD_CARDS_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_triple_triad_card_id {
                        self.toggle_triple_triad_card(i).await;
                    }
                }
                Task::ToggleAdventure { id } => {
                    self.toggle_adventure(*id, false).await;
                }
                Task::ToggleAdventureAll {} => {
                    let max_adventure_id = ADVENTURE_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_adventure_id {
                        if i == 0 {
                            self.toggle_adventure(i, true).await;
                        } else {
                            self.toggle_adventure(i, false).await;
                        }
                    }
                }
                Task::ToggleCutsceneSeen { id } => {
                    self.toggle_cutscene_seen(*id).await;
                }
                Task::ToggleCutsceneSeenAll {} => {
                    let max_cutscene_seen_id = CUTSCENE_SEEN_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_cutscene_seen_id {
                        self.toggle_cutscene_seen(i).await;
                    }
                }
                Task::ToggleMinion { id } => {
                    self.toggle_minion(*id).await;
                }
                Task::ToggleMinionAll {} => {
                    let max_minion_id = MINION_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_minion_id {
                        self.toggle_minion(i).await;
                    }
                }
                Task::ToggleAetherCurrent { id } => {
                    self.toggle_aether_current(*id).await;
                }
                Task::ToggleAetherCurrentAll {} => {
                    // TODO: seems like server has issues after executing it, but when you login back after being disconnected, seems to be alright?
                    let max_aether_current_id = AETHER_CURRENT_BITMASK_SIZE as u32 * 8;

                    for i in 2818048..(2818048 + max_aether_current_id) {
                        self.toggle_aether_current(i).await;
                    }
                }
                Task::ToggleAetherCurrentCompFlgSet { id } => {
                    self.toggle_aether_current_comp_flg_set(*id).await;
                }
                Task::ToggleAetherCurrentCompFlgSetAll {} => {
                    let max_aether_current_comp_flg_set_id =
                        AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE as u32 * 8;

                    // AetherCurrentCompFlgSet starts at Index 1
                    for i in 1..max_aether_current_comp_flg_set_id {
                        self.toggle_aether_current_comp_flg_set(i).await;
                    }
                }
                Task::SetRace { race } => {
                    let mut chara_details =
                        self.database.find_chara_make(self.player_data.content_id);
                    chara_details.chara_make.customize.race = *race;

                    self.database.set_chara_make(
                        self.player_data.content_id,
                        &chara_details.chara_make.to_json(),
                    );
                    self.respawn_player(false).await;
                }
                Task::SetTribe { tribe } => {
                    let mut chara_details =
                        self.database.find_chara_make(self.player_data.content_id);
                    chara_details.chara_make.customize.subrace = *tribe;

                    self.database.set_chara_make(
                        self.player_data.content_id,
                        &chara_details.chara_make.to_json(),
                    );
                    self.respawn_player(false).await;
                }
                Task::SetSex { sex } => {
                    let mut chara_details =
                        self.database.find_chara_make(self.player_data.content_id);
                    chara_details.chara_make.customize.gender = *sex;

                    self.database.set_chara_make(
                        self.player_data.content_id,
                        &chara_details.chara_make.to_json(),
                    );
                    self.respawn_player(false).await;
                }
                Task::SendSegment { segment } => {
                    self.send_segment(segment.clone()).await;
                }
                Task::StartTalkEvent {} => {
                    if let Some(event) = self.events.last_mut() {
                        event.talk(
                            ObjectTypeId {
                                object_id: ObjectId(self.player_data.actor_id),
                                object_type: ObjectTypeKind::None,
                            },
                            player,
                        );
                    }
                }
            }
        }
        player.queued_tasks.clear();

        // We have to do this because the onEnterTerritory may add new tasks
        if run_enter_territory {
            // Let the script now that it just loaded
            if let Some(event) = self.events.last_mut() {
                event.enter_territory(player);
            }
        }

        if run_finish_event {
            // Yield the last event again so it can pick up from nesting
            if let Some(event) = self.events.last_mut() {
                event.finish(0, &[], player);
            }
        }

        // We want to process again, since we probably added more tasks.
        // If we *don't* do this there is a pretty big delay before this can happen again.
        if run_enter_territory || run_finish_event {
            Box::pin(self.process_lua_player(player)).await;
        }
    }

    /// Reloads Global.lua
    pub fn reload_scripts(&mut self) {
        let mut lua = self.lua.lock();
        if let Err(err) = load_init_script(&mut lua, self.gamedata.clone()) {
            tracing::warn!("Failed to load Init.lua: {:?}", err);
        }
    }
}
