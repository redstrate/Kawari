required_rank = GM_RANK_DEBUG
command_sender = "[togglemount] "

function onCommand(args, player)
    local argc = #args
    if argc ~= 1 then
        printf(player, "Incorrect arguments given!")
        return
    end

    local id = args[1]
    if tonumber(id) then
        player:toggle_mount(id)
    else
        printf(player, "Incorrect arguments given!")
    end
end
