function doAction(player, in_combo)
    player:finish_dyeing() -- The client already sent us the information by now with the DyeInformation packet.

    return EffectsBuilder()
end
