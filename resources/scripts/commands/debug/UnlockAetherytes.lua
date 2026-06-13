required_rank = GM_RANK_DEBUG
command_sender = "[unlockaetherytes] "

function onCommand(player, args, name)
    player:unlock_aetheryte(1, 0)
    printf(player, "All aetherytes unlocked.")
end
