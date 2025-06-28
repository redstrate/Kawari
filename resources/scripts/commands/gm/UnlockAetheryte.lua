required_rank = GM_RANK_DEBUG
command_sender = "[unlockaetheryte] "

function onCommand(args, player)
    local on = args[1] -- TODO: reverse
    local id = args[2]

    player:unlock_aetheryte(on, id)
    printf(player, "Aetheryte(s) %s had their unlocked status changed!", id)
end
