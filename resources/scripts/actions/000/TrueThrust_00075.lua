POTENCY = 170

function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, DAMAGE_ELEMENT_UNASPECTED, player.parameters:calc_physical_damage(POTENCY))

    return effects
end
