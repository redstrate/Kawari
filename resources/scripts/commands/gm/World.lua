required_rank = GM_RANK_DEBUG
command_sender = "[world] "

function onCommand(player, args, name)
    printf(player, "%s (%d)", WORLD_NAME, WORLD_ID)
end
