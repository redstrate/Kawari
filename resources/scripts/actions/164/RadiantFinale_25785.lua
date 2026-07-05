-- Radiant Finale (BRD, ClassJob 23) - Level 90 ability
-- Duration: 15s
-- Effect: Increases damage of all party members by 5%
-- Requires: At least one song has been performed
-- Recast: 110s (CooldownGroup 14)
RADIANT_FINALE_STATUS = 2964
RADIANT_ENCORE_READY_STATUS = 3863
DURATION = 20.0
READY_DURATION = 30.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Apply Radiant Finale status to self (affects party via aura)
    effects:gain_effect_self(RADIANT_FINALE_STATUS, 0, DURATION)
    if player:get_level() >= 100 then
        effects:gain_effect_self(RADIANT_ENCORE_READY_STATUS, 0, READY_DURATION)
    end

    return effects
end
