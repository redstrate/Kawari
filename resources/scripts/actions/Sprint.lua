EFFECT_SPRINT = 50

function doAction(player)
    effects = EffectsBuilder()

    effects:gain_effect(EFFECT_SPRINT, 30, 20.0)

    return effects
end
