-- Army's Paeon (BRD, ClassJob 23) - Level 40 ability
-- Duration: 45s
-- Effect: Increases direct hit rate of all party members by 3%
-- When DoTs tick, has a chance to proc Army's Muse (skill speed up)
-- Adds 20 Soul Voice on activation
ARMYS_PAEON_STATUS = 2218
SONG_DURATION = 45.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Apply Army's Paeon song status to self
    effects:gain_effect_self(ARMYS_PAEON_STATUS, 0, SONG_DURATION)
    return effects
end
