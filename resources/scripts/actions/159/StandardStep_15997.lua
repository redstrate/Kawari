EFFECT_STANDARD_STEP = 1818

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect_self(EFFECT_STANDARD_STEP, 0, 15.0)

    return effects
end
