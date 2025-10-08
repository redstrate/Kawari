required_rank = GM_RANK_DEBUG
command_sender = "[togglecutsceneseen] "

function onCommand(args, player)
    local argc = #args
    if argc ~= 1 then
        printf(player, "Incorrect arguments given!")
        return
    end

    local id = args[1]

    if id == "all" then
        player:toggle_cutscene_seen_all()
    else
        player:toggle_cutscene_seen(tonumber(id))
    end
end
