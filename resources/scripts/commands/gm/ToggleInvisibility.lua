required_rank = GM_RANK_DEBUG
command_sender = "[invis] "

function onCommand(args, player)
    player:toggle_invisibility()
    printf(player, "Invisibility toggled.")
end
