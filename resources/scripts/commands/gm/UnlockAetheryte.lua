required_rank = GM_RANK_DEBUG
command_sender = "[unlockaetheryte] "

function onCommand(args, player)
    local on <const> = ~args[1] & 1  -- The client sends 1 for off and 0 for on, so we need to invert this for the rust side to work properly.
    local id <const> = args[2]

    player:unlock_aetheryte(on, id)
    printf(player, "Aetheryte(s) %s had their unlocked status changed!", id)
end
