required_rank = GM_RANK_DEBUG
command_sender = "[settribe] "

function onCommand(player, args, name)
    local tribe = args[1]

    local customize = player.chara_make.customize
    customize.tribe = tribe_from_repr(tribe)

    player:set_customize(customize)
    printf(player, "Set tribe to %s.", tribe)
end
