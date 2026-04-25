STATUS_HONED_REAVERS = 3772

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, 200)
    effects:gain_effect(STATUS_HONED_REAVERS, 0, 60.0)

    return effects
end
