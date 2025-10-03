function doAction(player)
    effects = EffectsBuilder()

    -- TODO: hardcoded to limsa for now, but it's also hardcoded in PlayerStatus
    player:warp_aetheryte(8)

    return effects
end
