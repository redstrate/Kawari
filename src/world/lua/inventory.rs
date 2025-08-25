use mlua::{UserData, UserDataFields};

use crate::inventory::{CurrencyStorage, EquippedStorage, GenericStorage, Inventory, Item};

impl UserData for Inventory {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("equipped", |_, this| Ok(this.equipped));
        fields.add_field_method_get("pages", |_, this| Ok(this.pages.clone()));
        fields.add_field_method_get("armoury_main_hand", |_, this| {
            Ok(this.armoury_main_hand.clone())
        });
        fields.add_field_method_get("armoury_head", |_, this| Ok(this.armoury_head.clone()));
        fields.add_field_method_get("armoury_body", |_, this| Ok(this.armoury_body.clone()));
        fields.add_field_method_get("armoury_hands", |_, this| Ok(this.armoury_hands.clone()));
        fields.add_field_method_get("armoury_legs", |_, this| Ok(this.armoury_legs.clone()));
        fields.add_field_method_get("armoury_feet", |_, this| Ok(this.armoury_feet.clone()));
        fields.add_field_method_get("armoury_off_hand", |_, this| {
            Ok(this.armoury_off_hand.clone())
        });
        fields.add_field_method_get("armoury_earring", |_, this| {
            Ok(this.armoury_earring.clone())
        });
        fields.add_field_method_get("armoury_necklace", |_, this| {
            Ok(this.armoury_necklace.clone())
        });
        fields.add_field_method_get("armoury_bracelet", |_, this| Ok(this.armoury_body.clone()));
        fields.add_field_method_get("armoury_rings", |_, this| Ok(this.armoury_rings.clone()));
        fields.add_field_method_get("armoury_soul_crystal", |_, this| {
            Ok(this.armoury_soul_crystal.clone())
        });
        fields.add_field_method_get("currency", |_, this| Ok(this.currency));
    }
}

impl UserData for Item {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("quantity", |_, this| Ok(this.quantity));
        fields.add_field_method_get("id", |_, this| Ok(this.id));
        fields.add_field_method_get("condition", |_, this| Ok(this.condition));
        fields.add_field_method_get("glamour_catalog_id", |_, this| Ok(this.condition));
    }
}

impl UserData for CurrencyStorage {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("gil", |_, this| Ok(this.gil));
    }
}

impl<const N: usize> UserData for GenericStorage<N> {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("slots", |_, this| Ok(this.slots.clone()));
    }
}

impl UserData for EquippedStorage {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("main_hand", |_, this| Ok(this.main_hand));
        fields.add_field_method_get("off_hand", |_, this| Ok(this.off_hand));
        fields.add_field_method_get("head", |_, this| Ok(this.head));
        fields.add_field_method_get("body", |_, this| Ok(this.body));
        fields.add_field_method_get("hands", |_, this| Ok(this.hands));
        fields.add_field_method_get("belt", |_, this| Ok(this.belt));
        fields.add_field_method_get("legs", |_, this| Ok(this.legs));
        fields.add_field_method_get("feet", |_, this| Ok(this.feet));
        fields.add_field_method_get("ears", |_, this| Ok(this.ears));
        fields.add_field_method_get("neck", |_, this| Ok(this.neck));
        fields.add_field_method_get("wrists", |_, this| Ok(this.wrists));
        fields.add_field_method_get("right_ring", |_, this| Ok(this.right_ring));
        fields.add_field_method_get("left_ring", |_, this| Ok(this.left_ring));
        fields.add_field_method_get("soul_crystal", |_, this| Ok(this.soul_crystal));
    }
}
