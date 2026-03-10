EFFECT_PELOTON = 1199

function doAction(player)
    effects = EffectsBuilder()
    -- TODO: what is in unk2?
    effects:gain_effect(EFFECT_PELOTON, 0, 30.0)

    return effects
end
