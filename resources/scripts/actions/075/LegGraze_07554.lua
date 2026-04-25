EFFECT_HEAVY = 14

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect(EFFECT_HEAVY, 40, 10.0)

    return effects
end
