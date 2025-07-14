EFFECT_SPRINT = 50

function doAction(player)
    effects = EffectsBuilder()

    effects:gain_effect(EFFECT_SPRINT)

    return effects
end
