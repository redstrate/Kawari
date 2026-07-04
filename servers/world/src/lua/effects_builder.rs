use mlua::{FromLua, Lua, UserData, UserDataMethods, Value};

use kawari::ipc::zone::{ActionEffect, DamageElement, DamageKind, DamageType, EffectKind};

/// A server-side enmity (hate) instruction produced by an action script.
///
/// Unlike [`ActionEffect`]s, these are *not* part of the network packets sent to the client.
/// They are resolved on the server (see `execute_action`) against the instance's hate lists
/// once the action's target is known.
#[derive(Clone, Debug)]
pub enum EnmityAction {
    /// Add a flat amount of enmity for the caster on the action's (hostile) target.
    Add { amount: u32 },
    /// Provoke: set the caster's enmity on the target to its current highest value plus one,
    /// putting the caster at the top of that target's hate list.
    Provoke,
    /// Shirk-like transfer: give the action's target `percent`% of the caster's current enmity
    /// on every enemy engaged with the caster. The caster keeps their own enmity.
    Transfer { percent: u32 },
}

/// A server-side job-gauge change produced by an action script, resolved in `execute_action`
/// against the caster's combat state. `index` selects the resource (job-specific; see the
/// resolver in action.rs) and `amount` is a signed delta — negative spends, positive grants.
#[derive(Clone, Copy, Debug)]
pub struct GaugeAction {
    pub index: u8,
    pub amount: i32,
}

/// The kind of periodic tick a DoT/HoT applies. Mirrors `world::TickEffectKind` but kept here to
/// avoid a dependency cycle; resolved into the real kind in `execute_action`.
#[derive(Clone, Copy, Debug)]
pub enum TickKind {
    DamageMagic,
    DamagePhysical,
    Heal,
    RestoreMp,
}

/// A server-side DoT/HoT instruction produced by an action script. The status itself is still sent
/// to the client as a normal `gain_effect`; this carries the per-tick potency the server needs to
/// resolve damage/healing every 3 seconds (the Status EXD sheet has no potency field). `on_self`
/// targets the caster (HoTs like Regen) instead of the action target.
#[derive(Clone, Copy, Debug)]
pub struct TickAction {
    pub effect_id: u16,
    pub param: u16,
    pub duration: f32,
    pub potency: u16,
    pub kind: TickKind,
    pub on_self: bool,
}

/// A server-side damage barrier produced by an action script. The status itself is still sent to
/// the client as a normal gain effect; this carries the absorb amount the server consumes before HP.
#[derive(Clone, Copy, Debug)]
pub struct BarrierAction {
    pub effect_id: u16,
    pub param: u16,
    pub duration: f32,
    pub amount: u32,
    pub on_self: bool,
}

#[derive(Clone, Debug, Default)]
pub struct EffectsBuilder {
    pub effects: Vec<ActionEffect>,
    /// Server-side enmity instructions to resolve once the action's target is known.
    pub enmity_actions: Vec<EnmityAction>,
    /// Server-side job-gauge changes to apply to the caster.
    pub gauge_actions: Vec<GaugeAction>,
    /// Server-side DoT/HoT registrations to resolve once the action's target is known.
    pub tick_actions: Vec<TickAction>,
    /// Server-side damage barriers to resolve once the action's target is known.
    pub barrier_actions: Vec<BarrierAction>,
}

impl UserData for EffectsBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "damage",
            |_, this, (damage_kind, damage_type, amount): (DamageKind, DamageType, u32)| {
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
        methods.add_method_mut(
            "gain_barrier",
            |_, this, (effect_id, param, duration, amount): (u16, u16, f32, u32)| {
                this.effects.push(ActionEffect {
                    kind: EffectKind::GainEffect {
                        unk1: 0,
                        unk2: 0,
                        unk3: 0,
                        effect_id,
                        duration,
                        param,
                    },
                });
                this.barrier_actions.push(BarrierAction {
                    effect_id,
                    param,
                    duration,
                    amount,
                    on_self: false,
                });
                Ok(())
            },
        );
        methods.add_method_mut(
            "gain_barrier_self",
            |_, this, (effect_id, param, duration, amount): (u16, u16, f32, u32)| {
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
                this.barrier_actions.push(BarrierAction {
                    effect_id,
                    param,
                    duration,
                    amount,
                    on_self: true,
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
        methods.add_method_mut("heal", |_, this, amount: u32| {
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
        methods.add_method_mut("summon_demi", |_, this, _: ()| {
            this.effects.push(ActionEffect {
                kind: EffectKind::SummonDemi {
                    unk: [0, 0, 0, 0, 0, 1, 0],
                },
            });
            Ok(())
        });
        methods.add_method_mut("execute_combo", |_, this, sequence: u8| {
            this.effects.push(ActionEffect {
                kind: EffectKind::ExecuteCombo {
                    sequence,
                    unk2: 0,
                    unk3: 0,
                    unk4: 0,
                    unk5: 128,
                    action_id: 0, // Filled in later
                },
            });
            Ok(())
        });
        // Add a flat amount of enmity for the caster on the action's target.
        methods.add_method_mut("add_enmity", |_, this, amount: u32| {
            this.enmity_actions.push(EnmityAction::Add { amount });
            Ok(())
        });
        // Provoke: become the top of the target's hate list.
        methods.add_method_mut("provoke", |_, this, _: ()| {
            this.enmity_actions.push(EnmityAction::Provoke);
            Ok(())
        });
        // Shirk: transfer `percent`% of the caster's enmity to the action's target.
        methods.add_method_mut("transfer_enmity", |_, this, percent: u32| {
            this.enmity_actions.push(EnmityAction::Transfer { percent });
            Ok(())
        });
        // Generic job-gauge change: modify_gauge(index, amount). `index` selects the resource
        // (job-specific), `amount` is a signed delta — negative spends, positive grants. The value
        // is clamped to the resource's valid range server-side (see action.rs).
        methods.add_method_mut("modify_gauge", |_, this, (index, amount): (u8, i32)| {
            this.gauge_actions.push(GaugeAction { index, amount });
            Ok(())
        });
        // DoT on the action's target: applies the status (icon/duration on the client) AND registers
        // a magical damage-over-time tick of `potency` resolved server-side every 3 seconds.
        methods.add_method_mut(
            "gain_dot",
            |_, this, (effect_id, param, duration, potency): (u16, u16, f32, u16)| {
                this.effects.push(ActionEffect {
                    kind: EffectKind::GainEffect {
                        unk1: 0,
                        unk2: 0,
                        unk3: 0,
                        effect_id,
                        duration,
                        param,
                    },
                });
                this.tick_actions.push(TickAction {
                    effect_id,
                    param,
                    duration,
                    potency,
                    kind: TickKind::DamageMagic,
                    on_self: false,
                });
                Ok(())
            },
        );
        // Physical DoT variant of `gain_dot`.
        methods.add_method_mut(
            "gain_dot_physical",
            |_, this, (effect_id, param, duration, potency): (u16, u16, f32, u16)| {
                this.effects.push(ActionEffect {
                    kind: EffectKind::GainEffect {
                        unk1: 0,
                        unk2: 0,
                        unk3: 0,
                        effect_id,
                        duration,
                        param,
                    },
                });
                this.tick_actions.push(TickAction {
                    effect_id,
                    param,
                    duration,
                    potency,
                    kind: TickKind::DamagePhysical,
                    on_self: false,
                });
                Ok(())
            },
        );
        // HoT on the caster (e.g. Regen): applies the status AND registers a heal-over-time tick of
        // `potency` resolved server-side every 3 seconds.
        methods.add_method_mut(
            "gain_hot",
            |_, this, (effect_id, param, duration, potency): (u16, u16, f32, u16)| {
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
                this.tick_actions.push(TickAction {
                    effect_id,
                    param,
                    duration,
                    potency,
                    kind: TickKind::Heal,
                    on_self: true,
                });
                Ok(())
            },
        );
        // MP refresh on the caster (e.g. Lucid Dreaming): applies the status AND registers a fixed
        // per-tick MP restore resolved server-side every 3 seconds.
        methods.add_method_mut(
            "gain_mp_refresh",
            |_, this, (effect_id, param, duration, amount): (u16, u16, f32, u16)| {
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
                this.tick_actions.push(TickAction {
                    effect_id,
                    param,
                    duration,
                    potency: amount,
                    kind: TickKind::RestoreMp,
                    on_self: true,
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
