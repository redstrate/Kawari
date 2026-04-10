required_rank = GM_RANK_DEBUG
command_sender = "[invis] "

function onCommand(player, args, name)
    player:toggle_invisibility()
    printf(player, "Invisibility toggled.")
end
