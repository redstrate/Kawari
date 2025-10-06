required_rank = GM_RANK_DEBUG
command_sender = "[toggleminion] "

function onCommand(args, player)
    local id = args[1]

    if id == "all" then
        player:toggle_minion_all()
    else
        player:toggle_minion(tonumber(id))
    end
end
