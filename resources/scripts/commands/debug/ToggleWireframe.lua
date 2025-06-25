required_rank = GM_RANK_DEBUG
sender = "[wireframe] "

function onCommand(args, player)
    local usage = "\nThis command allows the user to view the world in wireframe mode."

    player:toggle_wireframe()
    printf(player, "Wireframe mode toggled.")
end
