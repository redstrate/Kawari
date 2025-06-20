required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    player:set_position({ x = tonumber(parts[1]), y = tonumber(parts[2]), z = tonumber(parts[3]) }, 0)
end
