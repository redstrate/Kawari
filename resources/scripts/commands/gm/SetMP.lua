required_rank = GM_RANK_DEBUG
command_sender = "[setmp] "

function onCommand(player, args, name)
    local mp = args[1]

    player:set_mp(mp)
    printf(player, "Set MP to %s.", mp)
end
