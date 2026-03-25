use std::collections::{HashMap, VecDeque};

use crate::inventory::Item;

const BUYBACK_LIST_SIZE: usize = 10;
const BUYBACK_PARAM_COUNT: usize = 22;

#[derive(Clone, Debug, Default)]
pub struct BuyBackList {
    list: HashMap<u32, VecDeque<Item>>,
}

impl BuyBackList {
    pub fn push_item(&mut self, shop_id: u32, item: Item) {
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

    pub fn get_buyback_item(&self, shop_id: u32, index: u32) -> Option<&Item> {
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
                params[offset] = item.item_id;
                params[offset + 1] = item.quantity;
                params[offset + 2] = item.price_low;
                params[offset + 5] = shop_id;
                params[offset + 8] = item.condition as u32;
                // TODO: Fill in the rest of the information as it becomes known
                offset += BUYBACK_PARAM_COUNT;
            }
        }

        params
    }
}
