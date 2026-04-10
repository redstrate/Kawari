required_rank = GM_RANK_DEBUG
command_sender = "[toggleaethercurrentcompflgset] "

function onCommand(player, args, name)
    local argc = #args
    if argc ~= 1 then
        printf(player, "Incorrect arguments given!")
        return
    end

    local id = args[1]

    if id == "all" then
        player:toggle_aether_current_comp_flg_set_all()
        printf(player, "All aether currents unlocked, please log in again!")
    else
        player:toggle_aether_current_comp_flg_set(tonumber(id))
    end
end
