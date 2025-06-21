required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    local argc = table.getn(parts)
    local sender = "[wireframe] "
    local usage = "\nThis command allows the user to view the world in wireframe mode."

    player:toggle_wireframe()
    player:send_message(sender.."Wireframe mode toggled.")
end
