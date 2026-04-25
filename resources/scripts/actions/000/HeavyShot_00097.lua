POTENCY = 160

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, player.parameters:calc_physical_damage(POTENCY))

    return effects
end
