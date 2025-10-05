required_rank = GM_RANK_DEBUG
command_sender = "[toggleglassesstyle] "

function onCommand(args, player)
    local id = args[1]

    if id == "all" then
        player:toggle_glasses_style_all()
    else
        player:toggle_glasses_style(tonumber(id))
    end
end
