EFFECT_PHYSICAL_DAMAGE_UP = 53

function doAction(player)
    -- TODO: give to other enemies in range, this is an AoE

    effects = EffectsBuilder()
    effects:gain_effect(EFFECT_PHYSICAL_DAMAGE_UP, 0, 15.0)

    return effects
end
