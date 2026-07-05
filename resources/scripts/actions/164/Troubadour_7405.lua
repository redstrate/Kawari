-- Troubadour (BRD, ClassJob 23) - Level 62 ability
-- Duration: 15s
-- Effect: Reduces damage taken by all party members by 10%
-- Recast: 90s (CooldownGroup 21)
TROUBADOUR_STATUS = 1934
DURATION = 15.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Apply Troubadour status to self (affects party via aura)
    effects:gain_effect_self(TROUBADOUR_STATUS, 0, DURATION)

    return effects
end
