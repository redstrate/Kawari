EFFECT_PELOTON = 1199

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect(EFFECT_PELOTON, 20, 30.0)

    return effects
end
