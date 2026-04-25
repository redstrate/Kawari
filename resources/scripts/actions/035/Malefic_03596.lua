POTENCY = 150

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_MAGIC, player.parameters:calc_magical_damage(POTENCY))

    return effects
end
