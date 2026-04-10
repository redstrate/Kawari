required_rank = GM_RANK_DEBUG
command_sender = "[setrace] "

function onCommand(player, args, name)
    local race = args[1]

    player:set_race(race)
    printf(player, "Set race to %s.", race)
end
