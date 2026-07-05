-- Mage's Ballad (BRD, ClassJob 23) - Level 30 ability
-- Duration: 45s
-- Effect: Increases critical hit rate of all party members by 2%
-- When DoTs tick, has a chance to proc Repertoire (Army's Muse for skill speed)
-- Adds 20 Soul Voice on activation
MAGES_BALLAD_STATUS = 2217
SONG_DURATION = 45.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Apply Mage's Ballad song status to self
    effects:gain_effect_self(MAGES_BALLAD_STATUS, 0, SONG_DURATION)
    return effects
end
