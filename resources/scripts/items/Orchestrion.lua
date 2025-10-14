-- TODO: send ACS for orchestrion, otherwise you have to log out and log in

function doAction(player, arg)
    effects = EffectsBuilder()

    player:gm_set_orchestrion(true, arg)

    return effects
end
