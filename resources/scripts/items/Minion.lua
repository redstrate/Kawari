function doAction(player, arg)
    effects = EffectsBuilder()

    player:toggle_minion(arg)

    return effects
end
