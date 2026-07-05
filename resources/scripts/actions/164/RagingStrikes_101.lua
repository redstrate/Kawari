-- Raging Strikes (BRD, ClassJob 23) - Level 4 ability
-- Duration: 20s
-- Effect: Increases damage by 15%
-- Recast: 120s (CooldownGroup 11)
RAGING_STRIKES_STATUS = 125
DURATION = 20.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Apply Raging Strikes status to self
    effects:gain_effect_self(RAGING_STRIKES_STATUS, 0, DURATION)

    return effects
end
