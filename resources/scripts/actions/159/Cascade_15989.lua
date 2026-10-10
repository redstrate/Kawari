-- Without the "Dynamic Dancer" trait
NORMAL_POTENCY = 200
-- With the "Dynamic Dancer trait"
ENHANCED_POTENCY = 220

function doAction(player, in_combo)
    local potency
    if player:has_trait(TRAIT_DYNAMIC_DANCER) then
        potency = ENHANCED_POTENCY
    else
        potency = NORMAL_POTENCY
    end

    effects = EffectsBuilder()
    effects:damage(DAMAGE_TYPE_SLASHING, player.parameters:calc_physical_damage(potency))

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
