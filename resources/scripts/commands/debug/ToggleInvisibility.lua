required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local sender = "[invis] "
    local usage = "\nThis command makes the user invisible to all other actors."

    player:toggle_invisibility()
    player:send_message(sender.."Invisibility toggled.")
end
