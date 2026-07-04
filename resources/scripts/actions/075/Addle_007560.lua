-- 昏乱 / Addle
EFFECT_ADDLE = 1203

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect(EFFECT_ADDLE, 0, 15.0)

    return effects
end
