permissions = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    player:set_classjob(parts[1])
end
