-- Battle Voice (BRD, ClassJob 23) - Level 50 ability
-- Duration: 15s
-- Effect: Increases critical hit rate of all party members by 20%
-- Recast: 120s (CooldownGroup 19)
BATTLE_VOICE_STATUS = 141
DURATION = 15.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Apply Battle Voice status to self (affects party via aura)
    effects:gain_effect_self(BATTLE_VOICE_STATUS, 0, DURATION)

    return effects
end
