required_rank = GM_RANK_DEBUG
command_sender = "[wireframe] "

function onCommand(player, args, name)
    player:toggle_wireframe()
    printf(player, "Wireframe mode toggled.")
end
