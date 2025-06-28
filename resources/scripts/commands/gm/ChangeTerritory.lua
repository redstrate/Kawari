required_rank = GM_RANK_DEBUG
command_sender = "[teri] "

function onCommand(args, player)
    local id = args[1]

    player:change_territory(id)
    printf(player, "Changing territory to %s.", id)
end
