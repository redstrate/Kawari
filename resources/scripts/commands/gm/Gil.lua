required_rank = GM_RANK_DEBUG
command_sender = "[gil] "

function onCommand(args, player)
    local amount = args[1]

    player:modify_currency(CURRENCY_GIL, amount)
    printf(player, "Added %s gil.", amount)
end
