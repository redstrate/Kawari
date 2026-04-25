EFFECT_DEVILMENT = 1825

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect(EFFECT_DEVILMENT, 0, 20.0)

    return effects
end
