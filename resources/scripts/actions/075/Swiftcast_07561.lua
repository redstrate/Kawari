-- 即刻咏唱 / Swiftcast
EFFECT_SWIFTCAST = 167

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect_self(EFFECT_SWIFTCAST, 0, 10.0)

    return effects
end
