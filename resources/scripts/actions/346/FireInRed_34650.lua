STATUS_AETHERHUES = 3675

function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_MAGIC, DAMAGE_ELEMENT_UNASPECTED, 380)
    effects:gain_effect(STATUS_AETHERHUES, 0, 30.0)

    return effects
end
