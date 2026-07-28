STATUS_AETHERHUES = 3675

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_TYPE_MAGIC, 380)
    effects:gain_effect_self(STATUS_AETHERHUES, 0, 30.0)

    return effects
end
