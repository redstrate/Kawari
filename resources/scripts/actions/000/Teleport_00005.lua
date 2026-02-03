function doAction(player)
    effects = EffectsBuilder()

    -- get the aetheryte they requested
    local id = player.teleport_query.aetheryte_id

    -- warp there
    player:warp_aetheryte(id)

    return effects
end
