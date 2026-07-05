-- Nature's Minne (BRD, ClassJob 23) - Level 66 ability
-- Duration: 15s
-- Effect: Increases HP recovery via healing actions for a party member by 20%
-- Recast: 90s (CooldownGroup 22)
NATURES_MINNE_STATUS = 1202
DURATION = 15.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Apply Nature's Minne status to target
    effects:gain_effect(NATURES_MINNE_STATUS, 0, DURATION)

    return effects
end
