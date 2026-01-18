required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local argc = #args
    if argc ~= 1 then
        printf(player, "Incorrect arguments given!")
        return
    end

    local id = args[1]
    if tonumber(id) then
        player:unlock_classjob(tonumber(args[1]))
    else
        printf(player, "Incorrect arguments given!")
        return
    end
end
