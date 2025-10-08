required_rank = GM_RANK_DEBUG
command_sender = "[togglecaughtspearfish] "

function onCommand(args, player)
    local argc = #args
    if argc ~= 1 then
        printf(player, "Incorrect arguments given!")
        return
    end

    local id = args[1]

    if id == "all" then
        player:toggle_caught_spearfish_all()
    else
        player:toggle_caught_spearfish(tonumber(id))
    end
end
