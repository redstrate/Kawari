required_rank = GM_RANK_DEBUG
command_sender = "[weather] "

function onCommand(player, args, name)
    local id = args[1]

    player:change_weather(id)
    printf(player, "Changing weather to %s.", id)
end
