required_rank = GM_RANK_DEBUG
command_sender = "[togglecaughtfish] "

function onCommand(args, player)
    local id = args[1]

    if id == "all" then
        player:toggle_caught_fish_all()
    else
        player:toggle_caught_fish(tonumber(id))
    end
end
