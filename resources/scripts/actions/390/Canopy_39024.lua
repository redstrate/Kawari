function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, 1000) -- TODO: placeholder

    return effects
end
