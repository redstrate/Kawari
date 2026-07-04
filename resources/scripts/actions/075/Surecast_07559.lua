-- 沉稳咏唱 / Surecast
EFFECT_SURECAST = 160

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect_self(EFFECT_SURECAST, 0, 6.0)

    return effects
end
