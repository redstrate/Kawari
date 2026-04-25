EFFECT_FEINT = 1195

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect(EFFECT_FEINT, 0, 10.0)

    return effects
end
