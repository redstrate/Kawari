function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_MAGIC, DAMAGE_ELEMENT_FIRE, 1000) -- TODO: placeholder

    return effects
end
