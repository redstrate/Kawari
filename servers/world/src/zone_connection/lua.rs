//! Translates tasks and handles other information from `LuaPlayer`.

use crate::{
    ItemInfoQuery, ToServer, ZoneConnection,
    inventory::{CurrencyStorage, Item},
    lua::{LuaPlayer, LuaTask, load_init_script},
};
use kawari::{
    common::{
        ContainerType, DirectorEvent, ERR_INVENTORY_ADD_FAILED, HandlerId, InstanceContentType,
        ObjectTypeId, ObjectTypeKind,
    },
    constants::{
        ADVENTURE_BITMASK_SIZE, AETHER_CURRENT_BITMASK_SIZE,
        AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE, BUDDY_EQUIP_BITMASK_SIZE,
        CAUGHT_FISH_BITMASK_SIZE, CAUGHT_SPEARFISH_BITMASK_SIZE, CHOCOBO_TAXI_STANDS_BITMASK_SIZE,
        CUTSCENE_SEEN_BITMASK_SIZE, GLASSES_STYLES_BITMASK_SIZE, MINION_BITMASK_SIZE,
        ORNAMENT_BITMASK_SIZE, TRIPLE_TRIAD_CARDS_BITMASK_SIZE,
    },
    ipc::zone::{ActorControlCategory, ActorControlSelf, ServerZoneIpcData, ServerZoneIpcSegment},
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
        let mut run_finish_event = false;

        let tasks = player.queued_tasks.clone();
        for task in &tasks {
            match task {
                LuaTask::ChangeTerritory {
                    zone_id,
                    exit_position,
                    exit_rotation,
                } => {
                    self.change_zone(*zone_id, *exit_position, *exit_rotation)
                        .await
                }
                LuaTask::SetRemakeMode(remake_mode) => {
                    let mut database = self.database.lock();
                    database.set_remake_mode(player.player_data.content_id, *remake_mode);
                }
                LuaTask::Warp { warp_id } => {
                    self.warp(*warp_id).await;
                }
                LuaTask::BeginLogOut => self.begin_log_out().await,
                LuaTask::FinishEvent { handler_id } => {
                    self.event_finish(*handler_id).await;
                    run_finish_event = true;
                }
                LuaTask::SetClassJob { classjob_id } => {
                    self.player_data.classjob_id = *classjob_id;
                    self.update_class_info().await;
                }
                LuaTask::WarpAetheryte { aetheryte_id } => {
                    self.warp_aetheryte(*aetheryte_id).await;
                }
                LuaTask::ReloadScripts => {
                    self.reload_scripts();
                }
                LuaTask::ToggleInvisibility { invisible } => {
                    self.toggle_invisibility(*invisible).await;
                }
                LuaTask::Unlock { id } => {
                    self.player_data.unlock.unlocks.set(*id);

                    self.actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::ToggleUnlock {
                            id: *id,
                            unlocked: true,
                        },
                    })
                    .await;
                }
                LuaTask::UnlockAetheryte { id, on } => {
                    let unlock_all = *id == 0;
                    if unlock_all {
                        for i in 1..239 {
                            if *on {
                                self.player_data.aetheryte.unlocked.set(i);
                            } else {
                                self.player_data.aetheryte.unlocked.clear(i);
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
                            self.player_data.aetheryte.unlocked.set(*id);
                        } else {
                            self.player_data.aetheryte.unlocked.clear(*id);
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
                LuaTask::SetLevel { level } => {
                    self.set_current_level(*level);
                    self.update_class_info().await;
                }
                LuaTask::ChangeWeather { id } => {
                    self.change_weather(*id).await;
                }
                LuaTask::ModifyCurrency {
                    id,
                    amount,
                    send_client_update,
                } => {
                    let slot = self.player_data.inventory.currency.get_item_for_id(*id);

                    if *amount > 0 {
                        slot.quantity = slot.quantity.saturating_add(*amount as u32);
                    } else {
                        slot.quantity = slot.quantity.saturating_sub(-(*amount) as u32);
                    }

                    if *send_client_update {
                        let slot = *slot;

                        let ipc =
                            ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
                                sequence: self.player_data.item_sequence,
                                dst_storage_id: ContainerType::Currency,
                                dst_container_index: CurrencyStorage::get_slot_for_id(*id),
                                dst_stack: slot.quantity,
                                dst_catalog_id: slot.id,
                                unk1: 1966080000,
                            });
                        self.send_ipc_self(ipc).await;
                    }
                }
                LuaTask::GmSetOrchestrion { value, id } => {
                    self.gm_set_orchestrion(*value, *id);
                }
                LuaTask::ToggleOrchestrion { id } => {
                    self.toggle_orchestrion(*id).await;
                }
                LuaTask::AddItem {
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
                                self.send_inventory().await;
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
                LuaTask::UnlockContent { id } => {
                    {
                        let mut game_data = self.gamedata.lock();
                        if let Some(instance_content_type) = game_data.find_type_for_content(*id) {
                            // Each id has to be subtracted by it's offset in the InstanceContent Excel sheet. For example, all guildheists start at ID 10000.
                            match instance_content_type {
                                InstanceContentType::Dungeon => {
                                    self.player_data
                                        .content
                                        .unlocked_dungeons
                                        .set(*id as u32 - 1);
                                }
                                InstanceContentType::Raid => {
                                    self.player_data
                                        .content
                                        .unlocked_raids
                                        .set(*id as u32 - 30001);
                                }
                                InstanceContentType::Guildhests => {
                                    self.player_data
                                        .content
                                        .unlocked_guildhests
                                        .set(*id as u32 - 10001);
                                }
                                InstanceContentType::Trial => {
                                    self.player_data
                                        .content
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
                LuaTask::UnlockAllContent {} => {
                    self.player_data.content.unlocked_special_content.set_all();
                    self.player_data.content.unlocked_raids.set_all();
                    self.player_data.content.unlocked_dungeons.set_all();
                    self.player_data.content.unlocked_guildhests.set_all();
                    self.player_data.content.unlocked_trials.set_all();
                    self.player_data
                        .content
                        .unlocked_crystalline_conflicts
                        .set_all();
                    self.player_data.content.unlocked_frontlines.set_all();
                    self.player_data.content.unlocked_misc_content.set_all();
                }
                LuaTask::UpdateBuyBackList { list } => {
                    self.player_data.buyback_list = list.clone();
                }
                LuaTask::AddExp { amount } => {
                    self.add_exp(*amount).await;
                }
                LuaTask::StartEvent {
                    actor_id,
                    event_id,
                    event_type,
                    event_arg,
                } => {
                    self.start_event(*actor_id, *event_id, *event_type, *event_arg, None, player)
                        .await;
                }
                LuaTask::SetInnWakeup { watched } => {
                    self.player_data.saw_inn_wakeup = *watched;
                }
                LuaTask::ToggleMount { id } => {
                    let order;
                    {
                        let mut game_data = self.gamedata.lock();
                        order = game_data.find_mount_order(*id).unwrap_or(0);
                    }

                    let should_unlock = self.player_data.unlock.mounts.toggle(*id);

                    self.actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::ToggleMountUnlock {
                            order: order as u32,
                            id: *id,
                            unlocked: should_unlock,
                        },
                    })
                    .await;
                }
                LuaTask::MoveToPopRange { id, fade_out } => {
                    // Fade out the screen if requested.
                    if *fade_out {
                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PrepareZoning {
                            log_message: 0,
                            target_zone: self.player_data.zone_id,
                            animation: 0,
                            param4: 0,
                            hide_character: 0,
                            fade_out: 1,
                            param_7: 1,
                            fade_out_time: 1,
                            unk1: 0,
                            unk2: 0,
                        });
                        self.send_ipc_self(ipc).await;
                    }

                    self.handle
                        .send(ToServer::MoveToPopRange(
                            self.id,
                            self.player_data.actor_id,
                            *id,
                            *fade_out,
                        ))
                        .await;
                }
                LuaTask::SetHP { hp } => {
                    self.handle
                        .send(ToServer::SetHP(self.id, self.player_data.actor_id, *hp))
                        .await;
                }
                LuaTask::SetMP { mp } => {
                    self.handle
                        .send(ToServer::SetMP(self.id, self.player_data.actor_id, *mp))
                        .await;
                }
                LuaTask::ToggleGlassesStyle { id } => {
                    self.toggle_glasses_style(*id).await;
                }
                LuaTask::ToggleGlassesStyleAll {} => {
                    let max_glasses_style_id = GLASSES_STYLES_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_glasses_style_id {
                        self.toggle_glasses_style(i).await;
                    }
                }
                LuaTask::ToggleOrnament { id } => {
                    self.toggle_ornament(*id).await;
                }
                LuaTask::ToggleOrnamentAll {} => {
                    let max_ornament_id = ORNAMENT_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_ornament_id {
                        self.toggle_ornament(i).await;
                    }
                }
                LuaTask::UnlockBuddyEquip { id } => {
                    self.unlock_buddy_equip(*id).await;
                }
                LuaTask::UnlockBuddyEquipAll {} => {
                    let max_buddy_equip_id = BUDDY_EQUIP_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_buddy_equip_id {
                        self.unlock_buddy_equip(i).await;
                    }
                }
                LuaTask::ToggleChocoboTaxiStand { id } => {
                    self.toggle_chocobo_taxi_stand(*id).await;
                }
                LuaTask::ToggleChocoboTaxiStandAll {} => {
                    let max_chocobo_taxi_stand_id = CHOCOBO_TAXI_STANDS_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_chocobo_taxi_stand_id {
                        self.toggle_chocobo_taxi_stand(i).await;
                    }
                }
                LuaTask::ToggleCaughtFish { id } => {
                    self.toggle_caught_fish(*id).await;
                }
                LuaTask::ToggleCaughtFishAll {} => {
                    let max_caught_fish_id = CAUGHT_FISH_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_caught_fish_id {
                        self.toggle_caught_fish(i).await;
                    }
                }
                LuaTask::ToggleCaughtSpearfish { id } => {
                    self.toggle_caught_spearfish(*id).await;
                }
                LuaTask::ToggleCaughtSpearfishAll {} => {
                    let max_caught_spearfish_id = CAUGHT_SPEARFISH_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_caught_spearfish_id {
                        self.toggle_caught_spearfish(i).await;
                    }
                }
                LuaTask::ToggleTripleTriadCard { id } => {
                    self.toggle_triple_triad_card(*id).await;
                }
                LuaTask::ToggleTripleTriadCardAll {} => {
                    let max_triple_triad_card_id = TRIPLE_TRIAD_CARDS_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_triple_triad_card_id {
                        self.toggle_triple_triad_card(i).await;
                    }
                }
                LuaTask::ToggleAdventure { id } => {
                    self.toggle_adventure(*id, false).await;
                }
                LuaTask::ToggleAdventureAll {} => {
                    let max_adventure_id = ADVENTURE_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_adventure_id {
                        if i == 0 {
                            self.toggle_adventure(i, true).await;
                        } else {
                            self.toggle_adventure(i, false).await;
                        }
                    }
                }
                LuaTask::ToggleCutsceneSeen { id } => {
                    self.toggle_cutscene_seen(*id).await;
                }
                LuaTask::ToggleCutsceneSeenAll {} => {
                    let max_cutscene_seen_id = CUTSCENE_SEEN_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_cutscene_seen_id {
                        self.toggle_cutscene_seen(i).await;
                    }
                }
                LuaTask::ToggleMinion { id } => {
                    self.toggle_minion(*id).await;
                }
                LuaTask::ToggleMinionAll {} => {
                    let max_minion_id = MINION_BITMASK_SIZE as u32 * 8;

                    for i in 0..max_minion_id {
                        self.toggle_minion(i).await;
                    }
                }
                LuaTask::ToggleAetherCurrent { id } => {
                    self.toggle_aether_current(*id).await;
                }
                LuaTask::ToggleAetherCurrentAll {} => {
                    // TODO: seems like server has issues after executing it, but when you login back after being disconnected, seems to be alright?
                    let max_aether_current_id = AETHER_CURRENT_BITMASK_SIZE as u32 * 8;

                    for i in 2818048..(2818048 + max_aether_current_id) {
                        self.toggle_aether_current(i).await;
                    }
                }
                LuaTask::ToggleAetherCurrentCompFlgSet { id } => {
                    self.toggle_aether_current_comp_flg_set(*id).await;
                }
                LuaTask::ToggleAetherCurrentCompFlgSetAll {} => {
                    let max_aether_current_comp_flg_set_id =
                        AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE as u32 * 8;

                    // AetherCurrentCompFlgSet starts at Index 1
                    for i in 1..max_aether_current_comp_flg_set_id {
                        self.toggle_aether_current_comp_flg_set(i).await;
                    }
                }
                LuaTask::SetRace { race } => {
                    {
                        let mut database = self.database.lock();
                        let mut chara_make = database.get_chara_make(self.player_data.content_id);
                        chara_make.customize.race = *race;

                        database.set_chara_make(self.player_data.content_id, &chara_make.to_json());
                    }
                    self.respawn_player(false).await;
                }
                LuaTask::SetTribe { tribe } => {
                    {
                        let mut database = self.database.lock();
                        let mut chara_make = database.get_chara_make(self.player_data.content_id);
                        chara_make.customize.subrace = *tribe;

                        database.set_chara_make(self.player_data.content_id, &chara_make.to_json());
                    }
                    self.respawn_player(false).await;
                }
                LuaTask::SetSex { sex } => {
                    {
                        let mut database = self.database.lock();
                        let mut chara_make = database.get_chara_make(self.player_data.content_id);
                        chara_make.customize.gender = *sex;

                        database.set_chara_make(self.player_data.content_id, &chara_make.to_json());
                    }
                    self.respawn_player(false).await;
                }
                LuaTask::SendSegment { segment } => {
                    self.send_segment(segment.clone()).await;
                }
                LuaTask::StartTalkEvent {} => {
                    if let Some(event) = self.events.last_mut() {
                        event.talk(
                            ObjectTypeId {
                                object_id: self.player_data.actor_id,
                                object_type: ObjectTypeKind::None,
                            },
                            player,
                        );
                    }
                }
                LuaTask::AcceptQuest { id } => {
                    self.accept_quest(*id).await;
                }
                LuaTask::FinishQuest { id } => {
                    // this means "all"
                    if *id == 65535 {
                        self.finish_all_quests().await;
                    } else {
                        self.finish_quest(*id).await;
                    }
                }
                LuaTask::GainStatusEffect {
                    effect_id,
                    effect_param,
                    duration,
                } => {
                    self.gain_effect(*effect_id, *effect_param, *duration).await;
                }
                LuaTask::RegisterForContent { content_id } => {
                    self.register_for_content([*content_id, 0, 0, 0, 0]).await;
                }
                LuaTask::CommenceDuty { director_id } => {
                    // Have the director commence the duty
                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(
                        ActorControlSelf {
                            category: ActorControlCategory::DirectorEvent {
                                handler_id: HandlerId(*director_id),
                                event: DirectorEvent::DutyCommence,
                                arg: 5400,
                                unk1: 0,
                            },
                        },
                    ));
                    self.send_ipc_self(ipc).await;

                    // Signal to the global server to commence the duty as well, since they need to update the entrance circle.
                    self.handle
                        .send(ToServer::CommenceDuty(self.id, self.player_data.actor_id))
                        .await;

                    // shit
                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(
                        ActorControlSelf {
                            category: ActorControlCategory::DirectorEvent {
                                handler_id: HandlerId(*director_id),
                                event: DirectorEvent::SetDutyTimeRemaining,
                                arg: 5399,
                                unk1: 0,
                            },
                        },
                    ));
                    self.send_ipc_self(ipc).await;
                }
                LuaTask::QuestSequence { id, sequence } => {
                    self.set_quest_sequence(*id, *sequence).await;
                }
                LuaTask::CancelQuest { id } => {
                    self.cancel_quest(*id).await;
                }
                LuaTask::IncompleteQuest { id } => {
                    // this means "all"
                    if *id == 65535 {
                        self.incomplete_all_quests().await;
                    } else {
                        self.incomplete_quest(*id).await;
                    }
                }
                LuaTask::Kill {} => {
                    // Signal to the global server to kill us.
                    self.handle
                        .send(ToServer::Kill(self.id, self.player_data.actor_id))
                        .await;
                }
                LuaTask::AbandonContent {} => {
                    // Signal to the global server to leave this content.
                    self.handle
                        .send(ToServer::LeaveContent(
                            self.id,
                            self.player_data.actor_id,
                            self.old_zone_id,
                            self.old_position,
                            self.old_rotation,
                        ))
                        .await;
                }
                LuaTask::SetHomepoint { homepoint } => {
                    self.player_data.aetheryte.homepoint = *homepoint as i32;

                    // Also update the client live
                    self.actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::SetHomepoint {
                            id: *homepoint as u32,
                        },
                    })
                    .await;
                }
                LuaTask::ReturnToHomepoint {} => {
                    self.warp_aetheryte(self.player_data.aetheryte.homepoint as u32)
                        .await;
                }
            }
        }
        player.queued_tasks.clear();

        if run_finish_event {
            // Yield the last event again so it can pick up from nesting
            // TODO: this makes no sense, and probably needs to be re-worked
            if let Some(event) = self.events.last() {
                self.event_finish(event.id).await;
            }
        }

        // We want to process again, since we probably added more tasks.
        // If we *don't* do this there is a pretty big delay before this can happen again.
        if run_finish_event {
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
