function doAction(player, arg)
    effects = EffectsBuilder()

    effects:play_vfx(arg)

    return effects
end
