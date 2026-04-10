required_rank = GM_RANK_DEBUG
command_sender = "[setlevel] "

function onCommand(player, args, name)
    local level = args[1]

    player:set_level(level)
    printf(player, "Set level to %s.", level)
end
