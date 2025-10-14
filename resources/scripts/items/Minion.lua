function doAction(player, arg)
    effects = EffectsBuilder()

    print(arg)

    player:toggle_minion(arg)

    return effects
end
