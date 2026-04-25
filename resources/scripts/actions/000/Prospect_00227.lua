EFFECT_PROSPECT = 225

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect_self(EFFECT_PROSPECT, 0, 0.0)

    return effects
end
