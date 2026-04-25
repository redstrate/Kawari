POTENCY = 100
COMBO_POTENCY = 160

function doAction(player, in_combo)
    effects = EffectsBuilder()

    local potency = POTENCY
    if in_combo then
        potency = COMBO_POTENCY
    end

    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, player.parameters:calc_physical_damage(potency))

    if in_combo then
        -- Silken Flow has a 50% chance
        local gain_silken_flow = math.random(0, 1)
        if gain_silken_flow == 1 then
            effects:gain_effect_self(EFFECT_SILKEN_FLOW, 0, 30.0)
        end
    end

    return effects
end
