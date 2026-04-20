// ! The Moogle Mail Delivery system.
use bstr::BString;

use super::social::fetch_entries;
use crate::{
    ItemInfoQuery, ToServer, ZoneConnection,
    inventory::{CrystalKind, CurrencyKind, Item},
};
use kawari::{
    common::{INVENTORY_ACTION_ACK_SHOP, LogMessageType},
    ipc::zone::{
        ActorControlCategory, AttachedItemInfo, LetterPreview, LetterType, MAX_ATTACHMENTS,
        MAX_FRIEND_LETTERS, MAX_MAIL, MAX_MAIL_ATTACHMENTS_STORAGE, MAX_REWARD_LETTERS,
        MAX_SYSTEM_LETTERS, MailItemInfo, OnlineStatus, ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

impl ZoneConnection {
    pub async fn send_mailbox_status(&mut self) {
        let mut unread_counter = 0;
        let mut friend_counter = 0;
        let mut reward_counter = 0;
        let mut system_counter = 0;
        let mut attachments_counter = 0;
        let total_mail;
        let mut has_gm_mail = false;
        {
            use LetterType;

            let mut db = self.database.lock();
            let letters = db.find_letter_previews(self.player_data.character.content_id as u64);
            total_mail = letters.len();
            for letter in letters {
                match letter.mail_type {
                    LetterType::Player => friend_counter += 1,
                    LetterType::Reward => reward_counter += 1,
                    LetterType::GM => {
                        system_counter += 1;
                        if !letter.read {
                            has_gm_mail = true;
                        }
                    }
                }
                if !letter.read {
                    unread_counter += 1;
                }

                attachments_counter += letter
                    .attached_items
                    .iter()
                    .filter(|i| i.item_id != 0)
                    .count() as u16;
            }
        }
        use std::cmp;

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::MailboxStatus {
            letters_sent_back: if total_mail > MAX_MAIL {
                (total_mail - MAX_MAIL) as i32
            } else {
                0
            },
            attachments_counter: cmp::min(attachments_counter, MAX_ATTACHMENTS),
            unread_counter,
            friend_counter: cmp::min(friend_counter, MAX_FRIEND_LETTERS),
            reward_counter: cmp::min(reward_counter, MAX_REWARD_LETTERS),
            system_counter: cmp::min(system_counter, MAX_SYSTEM_LETTERS),
            has_gm_mail,
            has_support_message: false, // TODO: We're unlikely to implement this due to the support desk requiring external game modifications...
        });

        self.send_ipc_self(ipc).await;
    }

    pub async fn send_letter_previews(&mut self, unk1: u8) {
        // Only refresh and reset state if our list is empty.
        if self.mail_results.is_empty() {
            let mut db = self.database.lock();
            self.mail_results =
                db.find_letter_previews(self.player_data.character.content_id as u64);
            self.mail_index = 0;
        }

        let current_index = self.mail_index as u16;
        let mut next_index = self.mail_index as u16;

        let letters = fetch_entries(
            &mut next_index,
            &mut self.mail_results,
            LetterPreview::COUNT,
            &mut self.mail_index,
        );

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::MailboxPreview {
            letters,
            next_index: next_index as u8,
            current_index: current_index as u8,
            unk: unk1,
        });

        self.send_ipc_self(ipc).await;
    }

    async fn send_letter_update(
        &mut self,
        result: u32,
        sender_content_id: u64,
        timestamp: u32,
        updated_items: [AttachedItemInfo; MAX_MAIL_ATTACHMENTS_STORAGE],
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LetterUpdate {
            unk_result: result, // TODO: What does this 0xDD mean?
            unk1: 0,
            sender_content_id,
            timestamp,
            updated_items,
            unk2: [0; 4],
        });

        self.send_ipc_self(ipc).await;
    }

    pub async fn send_letter(
        &mut self,
        recipient_content_id: u64,
        attached_items: [MailItemInfo; MAX_MAIL_ATTACHMENTS_STORAGE],
        message: BString,
    ) {
        let is_online;
        let recipient_info;
        let mut need_to_send_inventory = false;
        {
            let mut db = self.database.lock();
            let osm = db.determine_online_status_mask(recipient_content_id as i64);
            is_online = osm.has_status(OnlineStatus::Online);

            let mut items =
                crate::inventory::GenericStorage::<{ MAX_MAIL_ATTACHMENTS_STORAGE }>::default();

            for item in items.slots.iter_mut().zip(attached_items.iter()) {
                // TODO: Maybe perform stricter validation on the item id and quantity?
                if item.1.item_id != 0 && item.1.item_quantity > 0 {
                    let Some(player_item) = self
                        .player_data
                        .inventory
                        .get_item_mut(item.1.src_container, item.1.src_container_index)
                    else {
                        tracing::warn!(
                            "Client attempted to mail an item from an invalid container: {:#?}",
                            item.1.src_container
                        );
                        return;
                    };
                    *item.0 = *player_item;
                    *player_item = crate::inventory::Item::default();
                    need_to_send_inventory = true;
                }
            }

            db.add_letter_to_mailbox(
                self.player_data.character.content_id as u64,
                recipient_content_id,
                LetterType::Player,
                message,
                items,
            );

            recipient_info = db.find_character_ids(Some(recipient_content_id), None);
        }

        if need_to_send_inventory {
            self.send_inventory().await;
            self.send_inventory_ack(u32::MAX, INVENTORY_ACTION_ACK_SHOP as u16)
                .await;
        }

        self.send_letter_update(
            0xDD,
            0,
            0,
            [AttachedItemInfo::default(); MAX_MAIL_ATTACHMENTS_STORAGE],
        )
        .await;

        if is_online && let Some(recipient_info) = recipient_info {
            self.handle
                .send(ToServer::SendLetterTo(recipient_info.actor_id))
                .await;
        }
    }

    pub async fn view_letter(&mut self, sender_content_id: u64, timestamp: u32) {
        let letter;
        {
            let mut db = self.database.lock();
            letter = db.find_letter(
                self.player_data.character.content_id as u64,
                sender_content_id,
                timestamp,
            );
        }

        if let Some(letter) = letter {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Letter(letter));
            self.send_ipc_self(ipc).await;
        }
    }

    pub async fn delete_letter(&mut self, sender_content_id: u64, timestamp: u32) {
        {
            let mut db = self.database.lock();
            db.remove_letter(
                self.player_data.character.content_id as u64,
                sender_content_id,
                timestamp,
            );
        }

        self.send_letter_update(
            0x366,
            sender_content_id,
            timestamp,
            [AttachedItemInfo::default(); MAX_MAIL_ATTACHMENTS_STORAGE],
        )
        .await;

        self.send_mailbox_status().await;
    }

    pub async fn take_attachments_from_letter(&mut self, sender_content_id: u64, timestamp: u32) {
        let mut attachments;
        {
            let mut db = self.database.lock();
            attachments = db.find_letter_attachments(
                self.player_data.character.content_id as u64,
                sender_content_id,
                timestamp,
            );
        }

        let Some(attachments) = &mut attachments else {
            tracing::warn!(
                "Attempted to take attachments from letter with no attachments: sender content id {sender_content_id}, timestamp {timestamp}!"
            );
            return;
        };

        let mut taken_items = [AttachedItemInfo::default(); MAX_MAIL_ATTACHMENTS_STORAGE];

        for (index, item) in attachments.slots.iter_mut().enumerate() {
            if item.is_empty_slot() {
                continue;
            }

            let mut item_taken = false;

            {
                let mut gamedata = self.gamedata.lock();
                item.stack_size = gamedata
                    .get_item_info(ItemInfoQuery::ById(item.item_id))
                    .unwrap()
                    .stack_size;
            }

            // TODO: Should we enforce gil being in the last attachment slot only? It should never appear anywhere else, but this system should be able to handle it..
            // NOTE: We don't do saturated adds here because we don't want to put gil or crystals into the void. If the player's inventory is "full" according to stack_size, we should not accept the attachment!
            if let Some(currency_kind) = CurrencyKind::from_repr(item.item_id) {
                let slot = self
                    .player_data
                    .inventory
                    .currency
                    .get_item_for_id(currency_kind);
                if slot.quantity + item.quantity <= item.stack_size {
                    slot.quantity += item.quantity;
                    item_taken = true;
                }
            } else if let Some(crystal_kind) = CrystalKind::from_repr(item.item_id) {
                let slot = self
                    .player_data
                    .inventory
                    .crystals
                    .get_item_for_id(crystal_kind);
                if slot.quantity + item.quantity <= item.stack_size {
                    slot.quantity += item.quantity;
                    item_taken = true;
                }
            } else if self
                .player_data
                .inventory
                .add_in_next_free_slot(*item)
                .is_some()
            {
                // TODO: Respect client's Armoury Chest settings ("Store all newly obtained items in the Armoury Chest")
                item_taken = true;
            }
            if item_taken {
                taken_items[index].item_id = item.item_id;
                taken_items[index].item_quantity = item.quantity;

                *item = Item::default();
            }
        }

        // If none of the items could be taken, bail here.
        let amount_taken = taken_items
            .iter()
            .filter(|i| i.item_id != 0 && i.item_quantity > 0)
            .count();
        if amount_taken == 0 {
            self.actor_control_self(ActorControlCategory::LogMessage2 {
                log_message: LogMessageType::UnableToAcceptAttachmentInventoryFull as u32,
            })
            .await;
            tracing::warn!(
                "We were unable to take any attachments, the player's inventory is full!"
            );
            return;
        }

        // Update the database with the new item attachments state.
        {
            let mut db = self.database.lock();
            db.set_letter_attachments(
                self.player_data.character.content_id as u64,
                sender_content_id,
                timestamp,
                attachments.clone(),
            );
        }

        // Taking attachments can put items literally anywhere, so a full inventory sync is needed.
        self.send_inventory().await;
        self.send_inventory_ack(u32::MAX, INVENTORY_ACTION_ACK_SHOP as u16)
            .await;

        // If only some of the items could be taken, notify the client and then send the letter update.
        let remaining_items = attachments
            .slots
            .iter()
            .filter(|i| !i.is_empty_slot())
            .count();

        if amount_taken < remaining_items {
            self.actor_control_self(ActorControlCategory::LogMessage2 {
                log_message: LogMessageType::UnableToAcceptAttachmentInventoryFull as u32,
            })
            .await;
            tracing::warn!(
                "We were unable to take all of the attachments, the player's inventory is now full!"
            );
        }

        self.send_letter_update(0x24E, sender_content_id, timestamp, taken_items)
            .await;
    }
}
