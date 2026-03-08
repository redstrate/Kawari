POTENCY = 140

function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_MAGIC, DAMAGE_ELEMENT_UNASPECTED, player.parameters:calc_magical_damage(POTENCY))

    return effects
end
