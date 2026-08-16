POTENCY = 100

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_TYPE_PHYSICAL, player.parameters:calc_physical_damage(POTENCY))

    return effects
end
