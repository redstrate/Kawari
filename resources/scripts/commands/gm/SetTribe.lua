required_rank = GM_RANK_DEBUG
command_sender = "[settribe] "

function onCommand(args, player)
    local tribe = args[1]

    player:set_tribe(tribe)
    printf(player, "Set tribe to %s.", tribe)
end
