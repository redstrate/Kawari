EFFECT_ARMS_LENGTH = 1209

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect(EFFECT_ARMS_LENGTH, 100, 6.0)

    return effects
end
