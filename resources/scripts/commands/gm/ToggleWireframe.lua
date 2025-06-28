required_rank = GM_RANK_DEBUG
command_sender = "[wireframe] "

function onCommand(args, player)
    player:toggle_wireframe()
    printf(player, "Wireframe mode toggled.")
end
