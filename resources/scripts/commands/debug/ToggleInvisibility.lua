required_rank = GM_RANK_DEBUG
command_sender = "[invis] "

function onCommand(args, player)
    local usage = "\nThis command makes the user invisible to all other actors."

    player:toggle_invisibility()
    printf(player, "Invisibility toggled.")
end
