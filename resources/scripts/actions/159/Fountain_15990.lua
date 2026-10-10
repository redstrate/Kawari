-- Without the "Dynamic Dancer" trait
NORMAL_POTENCY = 100
NORMAL_COMBO_POTENCY = 260
-- With the "Dynamic Dancer trait"
ENHANCED_POTENCY = 120
ENHANCED_COMBO_POTENCY = 280

function doAction(player, in_combo)
    effects = EffectsBuilder()

    local potency
    if player:has_trait(TRAIT_DYNAMIC_DANCER) then
        if in_combo then
            potency = ENHANCED_COMBO_POTENCY
        else
            potency = ENHANCED_POTENCY
        end
    else
        if in_combo then
            potency = NORMAL_COMBO_POTENCY
        else
            potency = NORMAL_POTENCY
        end
    end

    effects:damage(DAMAGE_TYPE_SLASHING, player.parameters:calc_physical_damage(potency))

    if in_combo then
        -- Silken Flow has a 50% chance
        local gain_silken_flow = math.random(0, 1)
        if gain_silken_flow == 1 then
            effects:gain_effect_self(EFFECT_SILKEN_FLOW, 0, 30.0)
        end
    end

    return effects
end
