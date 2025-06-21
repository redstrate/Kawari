required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    local argc = table.getn(parts)
    local sender = "[invis] "
    local usage = "\nThis command makes the user invisible to all other actors."

    player:toggle_invisibility()
    player:send_message(sender.."Invisibility toggled.")
end
