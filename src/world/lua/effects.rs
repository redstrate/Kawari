use mlua::{FromLua, Lua, LuaSerdeExt, UserData, UserDataMethods, Value};

use crate::{
    common::INVALID_OBJECT_ID,
    ipc::zone::{ActionEffect, DamageElement, DamageKind, DamageType, EffectKind},
};

#[derive(Clone, Debug, Default)]
pub struct EffectsBuilder {
    pub effects: Vec<ActionEffect>,
}

impl UserData for EffectsBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("damage", |lua, this, (damage_kind, damage_type, damage_element, amount): (Value, Value, Value, u16)| {
            let damage_kind: DamageKind = lua.from_value(damage_kind).unwrap();
            let damage_type: DamageType = lua.from_value(damage_type).unwrap();
            let damage_element: DamageElement = lua.from_value(damage_element).unwrap();

            this.effects.push(ActionEffect {
                kind: EffectKind::Damage {
                    damage_kind,
                    damage_type,
                    damage_element,
                    bonus_percent: 0,
                    unk3: 0,
                    unk4: 0,
                    amount,
                },
            });
            Ok(())
        });
        methods.add_method_mut(
            "gain_effect",
            |_, this, (effect_id, param, duration): (u16, u16, f32)| {
                this.effects.push(ActionEffect {
                    kind: EffectKind::Unk1 {
                        unk1: 0,
                        unk2: 7728,
                        effect_id,
                        duration,
                        param,
                        source_actor_id: INVALID_OBJECT_ID,
                    },
                });
                Ok(())
            },
        );
    }
}

impl FromLua for EffectsBuilder {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => unreachable!(),
        }
    }
}
