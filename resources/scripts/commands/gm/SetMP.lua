required_rank = GM_RANK_DEBUG
command_sender = "[setmp] "

function onCommand(args, player)
    local mp = args[1]

    player:set_mp(mp)
    printf(player, "Set MP to %s.", mp)
end
