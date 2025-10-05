required_rank = GM_RANK_DEBUG
command_sender = "[sethp] "

function onCommand(args, player)
    local hp = args[1]

    player:set_hp(hp)
    printf(player, "Set HP to %s.", hp)
end
