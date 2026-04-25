EFFECT_SHIELD_SAMBA = 1826

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect_self(EFFECT_SHIELD_SAMBA, 0, 15.0)

    return effects
end
