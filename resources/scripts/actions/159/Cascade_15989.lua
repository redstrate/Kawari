POTENCY = 220

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_TYPE_SLASHING, player.parameters:calc_physical_damage(POTENCY))

    -- Undocumented, but you don't gain Silken Symmetry from Cascade until level 20
    if player.level >= 20 then
        -- Silken Symmetry has a 50% chance
        local gain_silken_symmetry = math.random(0, 1)
        if gain_silken_symmetry == 1 then
            effects:gain_effect_self(EFFECT_SILKEN_SYMMETRY, 0, 30.0)
        end
    end

    return effects
end
