function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, DAMAGE_ELEMENT_UNASPECTED, 150)

    return effects
end
