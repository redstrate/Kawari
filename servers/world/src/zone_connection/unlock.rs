//! Utility functions for managing unlock data and bitmasks.

use crate::ZoneConnection;
use kawari::{
    common::value_to_flag_byte_index_value,
    ipc::zone::{ActorControlCategory, ActorControlSelf},
};

impl ZoneConnection {
    pub async fn toggle_orchestrion(&mut self, orchestrion_id: u32) {
        let should_unlock = self
            .player_data
            .unlock
            .orchestrion_rolls
            .toggle(orchestrion_id);

        let mut item_id = 0;

        if should_unlock {
            {
                let mut game_data = self.gamedata.lock();
                item_id = game_data
                    .find_orchestrion_item_id(orchestrion_id)
                    .unwrap_or(0);
            }
        }

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleOrchestrionUnlock {
                song_id: orchestrion_id,
                unlocked: should_unlock,
                item_id,
            },
        })
        .await;
    }

    pub async fn toggle_glasses_style(&mut self, glasses_style_id: u32) {
        let should_unlock = self
            .player_data
            .unlock
            .glasses_styles
            .toggle(glasses_style_id);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleGlassesStyleUnlock {
                id: glasses_style_id,
                unlocked: should_unlock,
            },
        })
        .await;
    }

    pub async fn toggle_ornament(&mut self, ornament_id: u32) {
        let should_unlock = self.player_data.unlock.ornaments.toggle(ornament_id);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleOrnamentUnlock {
                id: ornament_id,
                unlocked: should_unlock,
            },
        })
        .await;
    }

    pub async fn unlock_buddy_equip(&mut self, buddy_equip_id: u32) {
        self.player_data
            .companion
            .unlocked_equip
            .set(buddy_equip_id);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::BuddyEquipUnlock { id: buddy_equip_id },
        })
        .await;
    }

    pub async fn toggle_chocobo_taxi_stand(&mut self, chocobo_taxi_stand_id: u32) {
        let should_unlock = self
            .player_data
            .unlock
            .chocobo_taxi_stands
            .toggle(chocobo_taxi_stand_id);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleChocoboTaxiStandUnlock {
                id: chocobo_taxi_stand_id,
                unlocked: should_unlock,
            },
        })
        .await;
    }

    pub async fn toggle_caught_fish(&mut self, caught_fish_id: u32) {
        let (value, index) = value_to_flag_byte_index_value(caught_fish_id);

        self.player_data.unlock.caught_fish.0[index as usize] ^= value;

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::SetCaughtFishBitmask {
                index: index as u32,
                value: self.player_data.unlock.caught_fish.0[index as usize] as u32,
            },
        })
        .await;
    }

    pub async fn toggle_caught_spearfish(&mut self, caught_spearfish_id: u32) {
        let (value, index) = value_to_flag_byte_index_value(caught_spearfish_id);

        self.player_data.unlock.caught_spearfish.0[index as usize] ^= value;

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::SetCaughtSpearfishBitmask {
                index: index as u32,
                value: self.player_data.unlock.caught_spearfish.0[index as usize] as u32,
            },
        })
        .await;
    }

    pub async fn toggle_triple_triad_card(&mut self, triple_triad_card_id: u32) {
        let should_unlock = self
            .player_data
            .unlock
            .triple_triad_cards
            .toggle(triple_triad_card_id);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleTripleTriadCardUnlock {
                id: triple_triad_card_id,
                unlocked: should_unlock,
            },
        })
        .await;
    }

    // TODO: make logic that determines if all_vistas_recorded should be true or false automatically
    pub async fn toggle_adventure(&mut self, adventure_id: u32, all_vistas_recorded: bool) {
        let should_unlock = self.player_data.unlock.adventures.toggle(adventure_id);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleAdventureUnlock {
                id: adventure_id + 2162688,
                all_vistas_recorded,
                unlocked: should_unlock,
            },
        })
        .await;
    }

    pub async fn toggle_cutscene_seen(&mut self, cutscene_id: u32) {
        let should_unlock = self.player_data.unlock.cutscene_seen.toggle(cutscene_id);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleCutsceneSeen {
                id: cutscene_id,
                unlocked: should_unlock,
            },
        })
        .await;
    }

    pub async fn toggle_minion(&mut self, minion_id: u32) {
        let should_unlock = self.player_data.unlock.minions.toggle(minion_id);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleMinionUnlock {
                minion_id,
                unlocked: should_unlock,
            },
        })
        .await;
    }

    pub async fn toggle_aether_current(&mut self, aether_current_id: u32) {
        let aether_current_set;
        {
            let mut game_data = self.gamedata.lock();
            aether_current_set = game_data.find_aether_current_set(aether_current_id as i32);
        }

        if let Some(aether_current_set_id) = aether_current_set {
            let should_unlock = self
                .player_data
                .aether_current
                .unlocked
                .toggle(aether_current_id - 2818048);

            if should_unlock {
                let currents_needed_for_zone;
                let screen_image_id;

                {
                    let mut game_data = self.gamedata.lock();

                    currents_needed_for_zone = game_data
                        .get_aether_currents_from_zone(aether_current_set_id)
                        .unwrap();

                    screen_image_id = game_data
                        .get_screenimage_from_aether_current_comp_flg_set(aether_current_set_id)
                        .unwrap();
                }

                let mut zone_complete = true;

                for current_needed in currents_needed_for_zone {
                    let current_unlocked = self
                        .player_data
                        .aether_current
                        .unlocked
                        .contains((current_needed - 2818048) as u32);

                    if !current_unlocked {
                        zone_complete = false;
                        break;
                    }
                }

                self.actor_control_self(ActorControlSelf {
                    category: ActorControlCategory::ToggleAetherCurrentUnlock {
                        id: aether_current_id,
                        attunement_complete: zone_complete,
                        padding: 0,
                        screen_image_id: screen_image_id as u16,
                        zone_id: aether_current_set_id as u8,
                        unk1: zone_complete,
                        show_flying_mounts_help: false,
                        remove_aether_current: false,
                    },
                })
                .await;
            } else {
                self.actor_control_self(ActorControlSelf {
                    category: ActorControlCategory::ToggleAetherCurrentUnlock {
                        id: aether_current_id,
                        attunement_complete: false,
                        padding: 0,
                        screen_image_id: 0,
                        zone_id: aether_current_set_id as u8,
                        unk1: false,
                        show_flying_mounts_help: false,
                        remove_aether_current: true,
                    },
                })
                .await;
            }
        }
    }

    pub async fn toggle_aether_current_comp_flg_set(
        &mut self,
        aether_current_comp_flg_set_id: u32,
    ) {
        // Because AetherCurrentCompFlgSet starts at Index 1, we need to adjust the mask so this gives the proper values
        let should_unlock = self
            .player_data
            .aether_current
            .comp_flg_set
            .toggle(aether_current_comp_flg_set_id - 1);

        let screen_image_id;
        {
            let mut game_data = self.gamedata.lock();
            screen_image_id = game_data
                .get_screenimage_from_aether_current_comp_flg_set(aether_current_comp_flg_set_id)
                .unwrap();
        }

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleAetherCurrentUnlock {
                id: 0xFFFFFFFF, // The client does a check, if (as of 7.31h) id is greater than 56, then no individual Aether Current logic is done. This, hopefully, lasts for long.
                attunement_complete: should_unlock,
                padding: 0,
                screen_image_id: screen_image_id as u16,
                zone_id: aether_current_comp_flg_set_id as u8,
                unk1: should_unlock,
                show_flying_mounts_help: false,
                remove_aether_current: !should_unlock,
            },
        })
        .await;
    }

    // Difference between this an toggle_orchestrion, is that this one doesn't send an ActorControlSelf
    // The GM command itself manages the unlock for the client, so it isn't needed here
    pub fn gm_set_orchestrion(&mut self, value: bool, orchestrion_id: u32) {
        if value {
            self.player_data
                .unlock
                .orchestrion_rolls
                .set(orchestrion_id);
        } else {
            self.player_data
                .unlock
                .orchestrion_rolls
                .clear(orchestrion_id);
        }
    }
}
