required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    player:set_position({ x = tonumber(args[1]), y = tonumber(args[2]), z = tonumber(args[3]) }, 0)
end
