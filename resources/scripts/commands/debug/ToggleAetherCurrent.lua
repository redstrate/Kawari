required_rank = GM_RANK_DEBUG
command_sender = "[toggleaethercurrent] "

function onCommand(args, player)
    local argc = #args
    if argc ~= 1 then
        printf(player, "Incorrect arguments given!")
        return
    end

    local id = args[1]

    if id == "all" then
        player:toggle_aether_current_all()
    else
        player:toggle_aether_current(tonumber(id))
    end
end
