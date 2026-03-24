required_rank = GM_RANK_DEBUG
command_sender = "[world] "

function onCommand(args, player)
    printf(player, "%s (%d)", WORLD_NAME, WORLD_ID)
end
