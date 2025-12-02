use crate::ipc::zone::StatusEffect;

#[derive(Debug, Default, Clone)]
pub struct StatusEffects {
    status_effects: Vec<StatusEffect>,
    dirty: bool,
}

impl StatusEffects {
    pub fn add(&mut self, effect_id: u16, effect_param: u16, duration: f32) {
        let status_effect = self.find_or_create_status_effect(effect_id, effect_param);
        status_effect.duration = duration;
        self.dirty = true
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

    pub fn remove(&mut self, effect_id: u16) {
        if let Some(i) = self
            .status_effects
            .iter()
            .position(|effect| effect.effect_id == effect_id)
        {
            self.status_effects.remove(i);
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
}

#[cfg(test)]
mod tests {
    use crate::common::INVALID_OBJECT_ID;

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
                source_actor_id: INVALID_OBJECT_ID
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
}
