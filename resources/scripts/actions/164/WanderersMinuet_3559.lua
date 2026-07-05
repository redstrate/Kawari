-- Wanderer's Minuet (BRD, ClassJob 23) - Level 52 ability
-- Duration: 45s
-- Effect: Increases damage of all party members by 2%
-- When DoTs tick, has a chance to proc Repertoire (Pitch Perfect ready)
-- Adds 20 Soul Voice on activation
WANDERERS_MINUET_STATUS = 2216
SONG_DURATION = 45.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Apply Wanderer's Minuet song status to self
    effects:gain_effect_self(WANDERERS_MINUET_STATUS, 0, SONG_DURATION)
    return effects
end
