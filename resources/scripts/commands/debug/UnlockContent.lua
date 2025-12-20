required_rank = GM_RANK_DEBUG
command_sender = "[unlockcontent] "

function onCommand(args, player)
    local argc = #args
    if argc ~= 1 then
        printf(player, "Incorrect arguments given!")
        return
    end

    local id = args[1]
    if tonumber(id) then
        player:unlock_content(id)
        printf(player, "Content %s unlocked!", id)
    elseif id == "all" then
        player:unlock_all_content()
        printf(player, "All content unlocked, please log in again!")
    else
        printf(player, "Incorrect arguments given!")
    end
end
