-- 醒梦 / Lucid Dreaming
EFFECT_LUCID_DREAMING = 1204
MP_PER_TICK = 550
DURATION = 21.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_mp_refresh(EFFECT_LUCID_DREAMING, 0, DURATION, MP_PER_TICK)

    return effects
end
