EFFECT_PROSPECT = 225

function doAction(player)
    effects = EffectsBuilder()

    effects:gain_effect(EFFECT_PROSPECT, 0, 0.0)

    return effects
end
