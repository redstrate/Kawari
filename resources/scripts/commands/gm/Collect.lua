required_rank = GM_RANK_DEBUG
command_sender = "[collect] "

function onCommand(args, player)
    local amount = tonumber(args[1])
    if player.gil >= amount then
        player:modify_currency(CURRENCY_GIL, -amount)
        printf(player, "Collected %s gil.", amount)
    else
        printf(player, "Player does not have that much gil to take! They only possess %s.", player.gil)
    end
end
