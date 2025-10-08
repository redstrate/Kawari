required_rank = GM_RANK_DEBUG
command_sender = "[togglechocobotaxistand] "

function onCommand(args, player)
    local argc = #args
    if argc ~= 1 then
        printf(player, "Incorrect arguments given!")
        return
    end

    local id = args[1]

    if id == "all" then
        player:toggle_chocobo_taxi_stand_all()
    else
        player:toggle_chocobo_taxi_stand(tonumber(id))
    end
end
