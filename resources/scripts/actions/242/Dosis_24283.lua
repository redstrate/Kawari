function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_MAGIC, DAMAGE_ELEMENT_UNASPECTED, 180)

    return effects
end
