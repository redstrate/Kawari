use super::{WorldDatabase, models, schema::mail::dsl::*, unixepoch};
use crate::inventory::GenericStorage;
use bstr::BString;
use diesel::prelude::*;
use kawari::ipc::zone::{
    AttachedItemInfo, LETTER_MSG_MAX_LENGTH, Letter, LetterPreview, LetterType,
    MAX_MAIL_ATTACHMENTS_STORAGE, PREVIEW_MSG_MAX_LENGTH,
};

impl WorldDatabase {
    /// Adds a letter from `my_content_id` to `their_content_id`'s mailbox. In reality, it's just a flat table with *everyone*'s mail.
    pub fn add_letter_to_mailbox(
        &mut self,
        my_content_id: u64,
        their_content_id: u64,
        letter_kind: kawari::ipc::zone::LetterType,
        letter_message: BString,
        attachments: crate::inventory::GenericStorage<MAX_MAIL_ATTACHMENTS_STORAGE>,
    ) {
        let next_id = if let Ok(highest) = mail
            .select(id)
            .order(id.desc())
            .first::<i64>(&mut self.connection)
        {
            highest + 1
        } else {
            1 // Start from a safe default if there are no letters.
        };

        let time = diesel::select(unixepoch())
            .get_result::<i64>(&mut self.connection)
            .unwrap();

        // Before inserting the message, truncate it to the max length just in case.
        let mut letter_message = letter_message;
        letter_message.truncate(LETTER_MSG_MAX_LENGTH);

        let letter = models::Mail {
            id: next_id,
            kind: letter_kind as i32,
            read: false,
            timestamp: time,
            recipient_content_id: their_content_id as i64,
            sender_content_id: my_content_id as i64,
            message: serde_json::to_string(&letter_message).unwrap(),
            attached_items: serde_json::to_string(&attachments).unwrap(),
        };

        diesel::insert_into(mail)
            .values(letter)
            .execute(&mut self.connection)
            .unwrap();
    }

    pub fn find_letter_previews(&mut self, for_content_id: u64) -> Vec<LetterPreview> {
        let mut letters = Vec::new();

        let Ok(all_mail) = mail
            .select(models::Mail::as_select())
            .filter(recipient_content_id.eq(for_content_id as i64))
            .order(timestamp.desc())
            .load(&mut self.connection)
        else {
            return vec![LetterPreview::default(); LetterPreview::COUNT];
        };
        for letter in all_mail {
            let mut preview = LetterPreview {
                sender_content_id: letter.sender_content_id as u64,
                timestamp: letter.timestamp as u32,
                read: letter.read,
                mail_type: LetterType::from_repr(letter.kind as usize).unwrap_or_default(),
                ..Default::default()
            };

            preview.sender_name = if let Some(sender_info) =
                self.find_character_ids(Some(letter.sender_content_id as u64), None)
            {
                sender_info.name.clone()
            } else if letter.sender_content_id as u64 == u64::MAX {
                // Cash shop/gift mails seem to use a content id of u64::MAX. GM mails possibly, too?
                "Kawari World Server".to_string()
            } else {
                "(Unable to retrieve)".to_string() // TODO: Can we localise this somehow?
            };

            // Adjust the preview message to be no longer than PREVIEW_MSG_MAX_LENGTH bytes.
            let full_message: BString =
                serde_json::from_str(&letter.message.clone()).unwrap_or_default();
            preview.message = full_message;
            preview.message.truncate(PREVIEW_MSG_MAX_LENGTH);

            let attachments: crate::inventory::GenericStorage<6> =
                serde_json::from_str(&letter.attached_items).unwrap_or_default();
            // Since letter previews don't use full Items, we need to "downscale" to what the client is expecting.
            for (index, item) in attachments.slots.iter().enumerate() {
                preview.attached_items[index] = AttachedItemInfo {
                    item_id: item.item_id,
                    item_quantity: item.quantity,
                    ..Default::default()
                };
            }
            letters.push(preview);
        }

        letters
    }

    pub fn find_letter(
        &mut self,
        for_content_id: u64,
        their_content_id: u64,
        the_timestamp: u32,
    ) -> Option<kawari::ipc::zone::Letter> {
        let Ok(the_mail) = mail
            .select(models::Mail::as_select())
            .filter(recipient_content_id.eq(for_content_id as i64))
            .filter(sender_content_id.eq(their_content_id as i64))
            .filter(timestamp.eq(the_timestamp as i64))
            .first::<models::Mail>(&mut self.connection)
        else {
            return None;
        };

        // Mark the letter as read.
        diesel::update(mail)
            .filter(recipient_content_id.eq(for_content_id as i64))
            .filter(sender_content_id.eq(their_content_id as i64))
            .filter(timestamp.eq(the_timestamp as i64))
            .set(read.eq(true))
            .execute(&mut self.connection)
            .unwrap();

        // The message body can be no longer than LETTER_MSG_MAX_LENGTH bytes. This is probably redundant due to insertion handling this, but if manual db edits are done, this should catch any issues.
        let mut the_message: BString =
            serde_json::from_str(&the_mail.message.clone()).unwrap_or_default();
        the_message.truncate(LETTER_MSG_MAX_LENGTH);

        let the_letter = Letter {
            sender_content_id: the_mail.sender_content_id as u64,
            timestamp: the_mail.timestamp as u32,
            message: the_message,
        };

        Some(the_letter)
    }

    pub fn find_letter_attachments(
        &mut self,
        for_content_id: u64,
        their_content_id: u64,
        the_timestamp: u32,
    ) -> Option<GenericStorage<MAX_MAIL_ATTACHMENTS_STORAGE>> {
        if let Ok(the_items) = mail
            .select(attached_items)
            .filter(recipient_content_id.eq(for_content_id as i64))
            .filter(sender_content_id.eq(their_content_id as i64))
            .filter(timestamp.eq(the_timestamp as i64))
            .first::<String>(&mut self.connection)
            && let Ok(the_attachments) = serde_json::from_str(&the_items)
        {
            return Some(the_attachments);
        }

        None
    }

    pub fn set_letter_attachments(
        &mut self,
        for_content_id: u64,
        their_content_id: u64,
        the_timestamp: u32,
        the_attachments: GenericStorage<MAX_MAIL_ATTACHMENTS_STORAGE>,
    ) {
        let the_items = serde_json::to_string(&the_attachments).unwrap();

        diesel::update(
            mail.filter(recipient_content_id.eq(for_content_id as i64))
                .filter(sender_content_id.eq(their_content_id as i64))
                .filter(timestamp.eq(the_timestamp as i64)),
        )
        .set(attached_items.eq(the_items))
        .execute(&mut self.connection)
        .unwrap();
    }

    /// Deletes a letter sent by `their_content_id` at a specific `timestamp`.
    pub fn remove_letter(
        &mut self,
        for_content_id: u64,
        their_content_id: u64,
        the_timestamp: u32,
    ) {
        diesel::delete(
            mail.filter(recipient_content_id.eq(for_content_id as i64))
                .filter(sender_content_id.eq(their_content_id as i64))
                .filter(timestamp.eq(the_timestamp as i64)),
        )
        .execute(&mut self.connection)
        .unwrap();
    }

    /// Removes *all* off the letters received by `for_content_id`.
    pub fn remove_all_letters(&mut self, for_content_id: u64) {
        diesel::delete(mail.filter(recipient_content_id.eq(for_content_id as i64)))
            .execute(&mut self.connection)
            .unwrap();
    }
}
