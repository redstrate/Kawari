STATUS_AETHERHUES = 3675

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_MAGIC, 380)
    effects:gain_effect(STATUS_AETHERHUES, 0, 30.0)

    return effects
end
