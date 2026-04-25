POTENCY = 220

EFFECT_SILKEN_SYMMETRY = 2693

function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, player.parameters:calc_physical_damage(POTENCY))

    -- Silken Symmetry has a 50% chance
    local gain_silken_symmetry = math.random(0, 1)
    if gain_silken_symmetry == 1 then
        effects:gain_effect_self(EFFECT_SILKEN_SYMMETRY, 0, 30.0)
    end

    return effects
end
