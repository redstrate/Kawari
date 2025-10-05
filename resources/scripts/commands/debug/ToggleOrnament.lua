required_rank = GM_RANK_DEBUG
command_sender = "[toggleornament] "

function onCommand(args, player)
    local id = args[1]

    if id == "all" then
        player:toggle_ornament_all()
    else
        player:toggle_ornament(tonumber(id))
    end
end
