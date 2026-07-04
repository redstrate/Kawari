-- 日光普照 / Lux Solaris (self heal)
POTENCY = 500

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:heal(player.parameters:calc_heal_amount(POTENCY))

    return effects
end
