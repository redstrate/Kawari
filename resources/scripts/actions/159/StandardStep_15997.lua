EFFECT_STANDARD_STEP = 1818

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect(EFFECT_STANDARD_STEP, 0, 15.0)

    return effects
end
