function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_BLUNT, DAMAGE_ELEMENT_UNASPECTED, 1000) -- TODO: placeholder

    return effects
end
