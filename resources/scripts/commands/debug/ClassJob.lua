required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    player:set_classjob(tonumber(args[1]))
end
