required_rank = GM_RANK_DEBUG
command_sender = "[getrest] "

function onCommand(args, player)
    printf(player, "%d EXP", player.rested_exp)
end
