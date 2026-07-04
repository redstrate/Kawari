-- 守护之光 / Radiant Aegis (self shield)
RADIANT_AEGIS = 2702
SHIELD_PERCENT = 20

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_barrier_self(RADIANT_AEGIS, 0, 30.0, math.floor(player.parameters:max_hp() * SHIELD_PERCENT / 100))

    return effects
end
