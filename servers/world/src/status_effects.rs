use kawari::common::ObjectId;
use kawari::ipc::zone::StatusEffect;

/// The kind of periodic (every-3-seconds) tick a status effect applies. Retail computes the tick
/// magnitude from the *action* that applied the status (the Status EXD sheet has no potency field),
/// so the potency is supplied by the action script and stored here, not derived from game data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TickEffectKind {
    /// Damage over time (magical). Resolved against the target's HP each tick.
    DamageMagic,
    /// Damage over time (physical).
    DamagePhysical,
    /// Heal over time.
    Heal,
    /// Fixed MP restoration over time.
    RestoreMp,
}

/// A periodic effect attached to a status. Lives alongside the wire [`StatusEffect`] (which only
/// carries id/param/duration/source) so the every-3s regen tick can resolve DoTs/HoTs without
/// changing the network format.
#[derive(Debug, Clone, Copy)]
pub struct TickEffect {
    /// The status this tick belongs to (so it's removed together with the status).
    pub effect_id: u16,
    pub kind: TickEffectKind,
    /// Per-tick potency, or raw MP amount for [`TickEffectKind::RestoreMp`].
    pub potency: u16,
    /// Who applied the DoT/HoT, for damage attribution.
    pub source_actor_id: ObjectId,
}

/// A server-side damage barrier attached to a visible status effect. The wire status carries the
/// icon/duration/source; this stores the remaining absorb pool.
#[derive(Debug, Clone, Copy)]
pub struct BarrierEffect {
    pub effect_id: u16,
    pub remaining: u32,
}

#[derive(Debug, Default, Clone)]
pub struct StatusEffects {
    status_effects: Vec<StatusEffect>,
    /// Periodic tick effects (DoT/HoT) keyed by their owning status id. Server-side only.
    tick_effects: Vec<TickEffect>,
    /// Damage barriers keyed by their owning status id. Server-side only.
    barriers: Vec<BarrierEffect>,
    dirty: bool,
}

impl StatusEffects {
    pub fn add(&mut self, effect_id: u16, effect_param: u16, duration: f32) {
        self.add_with_source(effect_id, effect_param, duration, ObjectId::default());
    }

    /// Like [`add`], but records who applied the status. The `source_actor_id` is written into the
    /// wire `StatusEffect` so the client attributes it correctly — a self-applied status (source ==
    /// the actor) shows a green timer, otherwise white. Without this the StatusEffectList reports
    /// source 0 while the accompanying GainEffect ACS reports the real source, and the client draws
    /// the status twice (one white, one green).
    pub fn add_with_source(
        &mut self,
        effect_id: u16,
        effect_param: u16,
        duration: f32,
        source_actor_id: ObjectId,
    ) {
        let status_effect = self.find_or_create_status_effect(effect_id, effect_param);
        status_effect.duration = duration;
        status_effect.source_actor_id = source_actor_id;
        self.dirty = true
    }

    /// Adds (or refreshes) a status effect that also ticks every 3 seconds (DoT/HoT). The wire
    /// status is added as usual; the periodic `kind`/`potency` is stored separately so the regen
    /// tick can resolve it. Re-applying the same status id replaces its tick effect (refresh).
    pub fn add_tick(
        &mut self,
        effect_id: u16,
        effect_param: u16,
        duration: f32,
        kind: TickEffectKind,
        potency: u16,
        source_actor_id: ObjectId,
    ) {
        self.add_with_source(effect_id, effect_param, duration, source_actor_id);
        self.tick_effects.retain(|t| t.effect_id != effect_id);
        self.tick_effects.push(TickEffect {
            effect_id,
            kind,
            potency,
            source_actor_id,
        });
    }

    /// Adds (or refreshes) a status effect that absorbs incoming damage until `amount` is consumed.
    /// Re-applying the same status id replaces its previous barrier pool.
    pub fn add_barrier(
        &mut self,
        effect_id: u16,
        effect_param: u16,
        duration: f32,
        amount: u32,
        source_actor_id: ObjectId,
        max_barrier_total: u32,
    ) {
        self.add_with_source(effect_id, effect_param, duration, source_actor_id);
        self.barriers.retain(|b| b.effect_id != effect_id);

        let available = max_barrier_total.saturating_sub(self.barrier_amount());
        let amount = amount.min(available);
        if amount > 0 {
            self.barriers.push(BarrierEffect {
                effect_id,
                remaining: amount,
            });
        }
        self.dirty = true;
    }

    /// All periodic tick effects currently active (DoT/HoT).
    pub fn tick_effects(&self) -> &[TickEffect] {
        &self.tick_effects
    }

    /// Total remaining barrier amount.
    pub fn barrier_amount(&self) -> u32 {
        self.barriers
            .iter()
            .fold(0u32, |sum, barrier| sum.saturating_add(barrier.remaining))
    }

    /// Shield percentage as expected by StatusEffectList/EffectResult packets.
    pub fn shield_percent(&self, max_hp: u32) -> u8 {
        if max_hp == 0 {
            return 0;
        }

        (((self.barrier_amount() as u64 * 100).div_ceil(max_hp as u64)).min(100)) as u8
    }

    /// Absorbs `damage` through active barriers and returns the leftover HP damage.
    pub fn absorb_damage(&mut self, damage: u32) -> u32 {
        if damage == 0 || self.barriers.is_empty() {
            return damage;
        }

        let mut remaining_damage = damage;
        let mut broke_barrier = false;

        for barrier in &mut self.barriers {
            if remaining_damage == 0 {
                break;
            }

            let absorbed = barrier.remaining.min(remaining_damage);
            if absorbed == 0 {
                continue;
            }

            barrier.remaining -= absorbed;
            remaining_damage -= absorbed;
            self.dirty = true;

            if barrier.remaining == 0 {
                broke_barrier = true;
            }
        }

        if broke_barrier {
            let broken_effect_ids: Vec<u16> = self
                .barriers
                .iter()
                .filter(|barrier| barrier.remaining == 0)
                .map(|barrier| barrier.effect_id)
                .collect();
            self.barriers.retain(|barrier| barrier.remaining > 0);
            self.status_effects
                .retain(|effect| !broken_effect_ids.contains(&effect.effect_id));
            self.tick_effects
                .retain(|tick| !broken_effect_ids.contains(&tick.effect_id));
            self.dirty = true;
        }

        remaining_damage
    }

    fn find_or_create_status_effect(
        &mut self,
        effect_id: u16,
        effect_param: u16,
    ) -> &mut StatusEffect {
        if let Some(i) = self
            .status_effects
            .iter()
            .position(|effect| effect.effect_id == effect_id)
        {
            &mut self.status_effects[i]
        } else {
            self.status_effects.push(StatusEffect {
                effect_id,
                param: effect_param,
                ..Default::default()
            });
            self.status_effects.last_mut().unwrap()
        }
    }

    pub fn get(&self, effect_id: u16) -> Option<StatusEffect> {
        self.status_effects
            .iter()
            .position(|effect| effect.effect_id == effect_id)
            .map(|i| self.status_effects[i])
    }

    /// Returns the slot index of a status by id, matching the layout sent in StatusEffectList. The
    /// client keys buffs by this slot, so packets referencing a status (e.g. EffectResult) must use
    /// the same index or the buff is drawn twice.
    pub fn position_of(&self, effect_id: u16) -> Option<usize> {
        self.status_effects
            .iter()
            .position(|effect| effect.effect_id == effect_id)
    }

    pub fn remove(&mut self, effect_id: u16) {
        if let Some(i) = self
            .status_effects
            .iter()
            .position(|effect| effect.effect_id == effect_id)
        {
            self.status_effects.remove(i);
            self.dirty = true;
        }
        self.tick_effects.retain(|t| t.effect_id != effect_id);
        let barrier_count = self.barriers.len();
        self.barriers.retain(|b| b.effect_id != effect_id);
        if self.barriers.len() != barrier_count {
            self.dirty = true;
        }
    }

    pub fn data(&self) -> &[StatusEffect] {
        &self.status_effects
    }

    /// If the list is dirty and must be propagated to the client
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn reset_dirty(&mut self) {
        self.dirty = false;
    }

    /// Number of status effects.
    pub fn len(&self) -> usize {
        self.status_effects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.status_effects.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_effects() {
        // Ensure sensible initial state
        let mut status_effects = StatusEffects::default();
        assert_eq!(status_effects.get(0), None);
        assert_eq!(status_effects.is_dirty(), false);

        // Add a status effect, check that it can be grabbed again, and that the dirty flag is set:
        status_effects.add(0, 0, 0.0);
        assert_eq!(
            status_effects.get(0),
            Some(StatusEffect {
                effect_id: 0,
                param: 0,
                duration: 0.0,
                source_actor_id: Default::default()
            })
        );
        assert_eq!(status_effects.is_dirty(), true);

        // Try resetting the dirty flag:
        status_effects.reset_dirty();
        assert_eq!(status_effects.is_dirty(), false);

        // Removing a status should mark it as dirty, and it should really be gone:
        status_effects.remove(0);
        assert_eq!(status_effects.get(0), None);
        assert_eq!(status_effects.is_dirty(), true);
    }

    #[test]
    fn test_barrier_absorbs_damage_and_removes_status_when_broken() {
        let mut status_effects = StatusEffects::default();
        status_effects.add_barrier(2702, 0, 30.0, 100, ObjectId(1), 1000);

        assert_eq!(status_effects.get(2702).unwrap().effect_id, 2702);
        assert_eq!(status_effects.barrier_amount(), 100);
        assert_eq!(status_effects.shield_percent(1000), 10);

        assert_eq!(status_effects.absorb_damage(40), 0);
        assert_eq!(status_effects.barrier_amount(), 60);
        assert!(status_effects.get(2702).is_some());

        assert_eq!(status_effects.absorb_damage(90), 30);
        assert_eq!(status_effects.barrier_amount(), 0);
        assert!(status_effects.get(2702).is_none());
    }

    #[test]
    fn test_barrier_total_is_capped_to_max_hp() {
        let mut status_effects = StatusEffects::default();
        status_effects.add_barrier(2702, 0, 30.0, 800, ObjectId(1), 1000);
        status_effects.add_barrier(297, 0, 30.0, 800, ObjectId(1), 1000);

        assert_eq!(status_effects.barrier_amount(), 1000);
        assert_eq!(status_effects.absorb_damage(1200), 200);
    }
}
