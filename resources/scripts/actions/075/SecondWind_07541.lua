POTENCY = 800

function doAction(player)
    effects = EffectsBuilder()
    effects:heal(player.parameters:calc_heal_amount(POTENCY))

    return effects
end
