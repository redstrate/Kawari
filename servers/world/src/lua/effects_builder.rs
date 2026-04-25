use mlua::{FromLua, Lua, UserData, UserDataMethods, Value};

use kawari::ipc::zone::{ActionEffect, DamageElement, DamageKind, DamageType, EffectKind};

#[derive(Clone, Debug, Default)]
pub struct EffectsBuilder {
    pub effects: Vec<ActionEffect>,
}

impl UserData for EffectsBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "damage",
            |_, this, (damage_kind, damage_type, amount): (DamageKind, DamageType, u16)| {
                this.effects.push(ActionEffect {
                    kind: EffectKind::Damage {
                        damage_kind,
                        damage_type,
                        damage_element: DamageElement::Unaspected, // Will be filled in later
                        bonus_percent: 0,
                        unk3: 0,
                        unk4: 0,
                        amount,
                    },
                });
                Ok(())
            },
        );
        methods.add_method_mut(
            "gain_effect",
            |_, this, (effect_id, param, duration): (u16, u16, f32)| {
                this.effects.push(ActionEffect {
                    kind: EffectKind::GainEffect {
                        unk1: 0,
                        unk2: 0,
                        unk3: 0,
                        effect_id,
                        duration,
                        param,
                        source_actor_id: Default::default(),
                    },
                });
                Ok(())
            },
        );
        methods.add_method_mut(
            "gain_effect_self",
            |_, this, (effect_id, param, duration): (u16, u16, f32)| {
                this.effects.push(ActionEffect {
                    kind: EffectKind::GainEffectSelf {
                        unk1: 0,
                        unk2: 0,
                        unk3: 0,
                        effect_id,
                        duration,
                        param,
                    },
                });
                Ok(())
            },
        );
        // TODO: if we ever decide to redo the effectbuilder to not directly push actioneffects, this should also be redone
        // so we can remove the param arg. Assuming there isn't effects with the same ID but different params?
        methods.add_method_mut(
            "lose_effect",
            |_, this, (effect_id, effect_param): (u16, u16)| {
                this.effects.push(ActionEffect {
                    kind: EffectKind::LoseEffect {
                        param: effect_param,
                        unk: [0; 3],
                        effect_id,
                    },
                });
                Ok(())
            },
        );
        methods.add_method_mut("heal", |_, this, amount: u16| {
            this.effects.push(ActionEffect {
                kind: EffectKind::Heal {
                    unk1: [0; 5],
                    amount,
                },
            });
            Ok(())
        });
        methods.add_method_mut("interrupt", |_, this, _: ()| {
            this.effects.push(ActionEffect {
                kind: EffectKind::InterruptAction {},
            });
            Ok(())
        });
        methods.add_method_mut("play_vfx", |_, this, effect_id: u16| {
            this.effects.push(ActionEffect {
                kind: EffectKind::PlayVFX {
                    unk: [0; 5],
                    effect_id,
                },
            });
            Ok(())
        });
        methods.add_method_mut("summon_pet", |_, this, _: ()| {
            this.effects.push(ActionEffect {
                kind: EffectKind::SummonPet {
                    unk: [0, 0, 0, 0, 128, 157, 0],
                },
            });
            Ok(())
        });
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
