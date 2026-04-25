EFFECT_BIND = 13

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect(EFFECT_BIND, 0, 10.0)

    return effects
end
