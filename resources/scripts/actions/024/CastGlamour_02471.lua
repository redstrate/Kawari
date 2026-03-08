function doAction(player)
    player:finish_casting_glamour() -- The client already sent us the glamour information by now with the PrepareCastGlamour CT.

    return EffectsBuilder()
end
