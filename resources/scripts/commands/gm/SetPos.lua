required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    -- for some reason the client gives us it to multiplied by 100
    player:set_position({ x = tonumber(args[1]) / 100, y = tonumber(args[2]) / 100, z = tonumber(args[3]) / 100 }, 0)
end
