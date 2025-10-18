function doAction(player)
    effects = EffectsBuilder()

    -- we have to send it with the param, so we need to fetch the effect first
    local effect = player:get_effect(EFFECT_TRANSFIGURATION)
    effects:lose_effect(EFFECT_TRANSFIGURATION, effect.param)

    return effects
end
