POTENCY = 450

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:heal(player.parameters:calc_heal_amount(POTENCY))

    return effects
end
