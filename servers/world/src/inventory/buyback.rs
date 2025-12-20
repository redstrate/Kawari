use std::collections::{HashMap, VecDeque};

use crate::ItemInfo;

const BUYBACK_LIST_SIZE: usize = 10;
const BUYBACK_PARAM_COUNT: usize = 22;

// TODO: Deprecate this type, Item can now be expanded to support everything we'll need
#[derive(Clone, Debug, Default)]
pub struct BuyBackItem {
    pub id: u32,
    pub quantity: u32,
    pub price_low: u32,
    // TODO: there are 22 total things the server keeps track of and sends back to the client, we should implement these!
    // Not every value is not fully understood but they appeared to be related to item quality, materia melds, the crafter's name (if applicable), spiritbond/durability, and maybe more.
    /// Fields beyond this comment are not part of the 22 datapoints the server sends to the client, but we need them for later item restoration.
    pub item_level: u16,
    pub stack_size: u32,
}

#[derive(Clone, Debug, Default)]
pub struct BuyBackList {
    list: HashMap<u32, VecDeque<BuyBackItem>>,
}

impl BuyBackList {
    pub fn push_item(&mut self, shop_id: u32, item: BuyBackItem) {
        let vec = self.list.entry(shop_id).or_default();
        vec.push_front(item);
        vec.truncate(BUYBACK_LIST_SIZE);
    }

    pub fn remove_item(&mut self, shop_id: u32, index: u32) {
        let Some(vec) = self.list.get_mut(&shop_id) else {
            tracing::warn!(
                "Attempting to remove an item from a BuyBackList that doesn't have any items! This is likely a bug!"
            );
            return;
        };

        vec.remove(index as usize);
    }

    pub fn get_buyback_item(&self, shop_id: u32, index: u32) -> Option<&BuyBackItem> {
        let vec = self.list.get(&shop_id)?;

        vec.get(index as usize)
    }

    pub fn as_scene_params(&mut self, shop_id: u32, shop_intro: bool) -> Vec<u32> {
        let mut params = Vec::<u32>::new();

        /* Adjust the params array to be the appropriate size based on what is happening.
         * The caller is responsible for editing the extra params, as our duty here is to convert our stored information
         * into u32s so the game client can digest it.
         * When the shop is first opened we allocate one extra parameter, and all other actions require 2.*/
        let mut offset: usize;
        if shop_intro {
            params.resize(BUYBACK_LIST_SIZE * BUYBACK_PARAM_COUNT + 1, 0u32);
            offset = 1;
        } else {
            params.resize(BUYBACK_LIST_SIZE * BUYBACK_PARAM_COUNT + 2, 0u32);
            offset = 2;
        }

        self.list.entry(shop_id).or_default();
        let shop_buyback_items = self.list.get(&shop_id).unwrap();
        if !shop_buyback_items.is_empty() {
            for item in shop_buyback_items {
                params[offset] = item.id;
                params[offset + 1] = item.quantity;
                params[offset + 2] = item.price_low;
                params[offset + 5] = shop_id;
                params[offset + 8] = 0x7530_0000; // TODO: What is this? It's not static either, it can change if items have melds or a crafter signature, so right now it's unknown.
                // TODO: Fill in the rest of the information as it becomes known
                offset += BUYBACK_PARAM_COUNT;
            }
        }

        params
    }
}

// TODO: Once BBItem is deprecated, remove this. This is a transitional impl as we migrate to using Item.
impl BuyBackItem {
    pub fn as_item_info(&self) -> ItemInfo {
        ItemInfo {
            id: self.id,
            item_level: self.item_level,
            stack_size: self.stack_size,
            price_low: self.price_low,
            ..Default::default()
        }
    }
}
