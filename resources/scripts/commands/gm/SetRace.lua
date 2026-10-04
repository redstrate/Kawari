required_rank = GM_RANK_DEBUG
command_sender = "[setrace] "

function onCommand(player, args, name)
    local race = args[1]

    local customize = player.chara_make.customize
    customize.race = race_from_repr(race)

    player:set_customize(customize)
    printf(player, "Set race to %s.", race)
end
