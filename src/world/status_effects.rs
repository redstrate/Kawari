use crate::ipc::zone::StatusEffect;

#[derive(Debug, Default, Clone)]
pub struct StatusEffects {
    pub status_effects: Vec<StatusEffect>,
    /// If the list is dirty and must be propagated to the client
    pub dirty: bool,
}

impl StatusEffects {
    pub fn add(&mut self, effect_id: u16, duration: f32) {
        let status_effect = self.find_or_create_status_effect(effect_id);
        status_effect.duration = duration;
        self.dirty = true
    }

    fn find_or_create_status_effect(&mut self, effect_id: u16) -> &mut StatusEffect {
        if let Some(i) = self
            .status_effects
            .iter()
            .position(|effect| effect.effect_id == effect_id)
        {
            &mut self.status_effects[i]
        } else {
            self.status_effects.push(StatusEffect {
                effect_id,
                ..Default::default()
            });
            self.status_effects.last_mut().unwrap()
        }
    }
}
