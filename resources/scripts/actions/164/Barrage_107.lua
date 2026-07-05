-- Barrage (BRD, ClassJob 23) - Level 38 ability
-- Duration: 10s
-- Effect: Next weaponskill deals damage 3 times
-- Recast: 120s (CooldownGroup 20)
-- Grants Hawk Eye status for Refulgent Arrow/Shadowbite
BARRAGE_STATUS = 128
HAWK_EYE_STATUS = 3861
RESONANT_ARROW_READY_STATUS = 3862
DURATION = 10.0
READY_DURATION = 30.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Apply Barrage status to self
    effects:gain_effect_self(BARRAGE_STATUS, 0, DURATION)
    -- Also grant Hawk Eye for Refulgent Arrow/Shadowbite
    effects:gain_effect_self(HAWK_EYE_STATUS, 0, READY_DURATION)
    if player:get_level() >= 96 then
        effects:gain_effect_self(RESONANT_ARROW_READY_STATUS, 0, READY_DURATION)
    end

    return effects
end
