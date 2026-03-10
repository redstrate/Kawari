EFFECT_HEAVY = 14

function doAction(player)
    effects = EffectsBuilder()
    -- TODO: add amount, it's under unk2 i think
    effects:gain_effect(EFFECT_HEAVY, 0, 10.0)

    return effects
end
