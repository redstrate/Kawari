function doAction(player, arg)
    effects = EffectsBuilder()

    player:toggle_orchestrion(arg)

    return effects
end
