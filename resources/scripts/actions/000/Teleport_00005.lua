function doAction(player, in_combo)
    -- get the aetheryte they requested
    local id = player.teleport_query.aetheryte_id

    -- warp there
    player:warp_aetheryte(id)

    return EffectsBuilder()
end
