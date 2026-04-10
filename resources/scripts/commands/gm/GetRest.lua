required_rank = GM_RANK_DEBUG
command_sender = "[getrest] "

function onCommand(player, args, name)
    printf(player, "%d EXP", player.rested_exp)
end
