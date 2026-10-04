required_rank = GM_RANK_DEBUG
command_sender = "[setsex] "

function onCommand(player, args, name)
    local sex = args[1]

    local customize = player.chara_make.customize
    customize.gender = gender_from_repr(sex)

    player:set_customize(customize)
    printf(player, "Set sex to %s.", sex)
end
