required_rank = GM_RANK_DEBUG
command_sender = "[setsex] "

function onCommand(player, args, name)
    local sex = args[1]

    player:set_sex(sex)
    printf(player, "Set sex to %s.", sex)
end
