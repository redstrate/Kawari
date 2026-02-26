required_rank = GM_RANK_DEBUG
command_sender = "[monies] "

function onCommand(args, player)
    local amount = 9999999

    player:modify_currency(CURRENCY_GIL, amount)
    player:modify_currency(CURRENCY_WOLF_MARK, amount)
    player:modify_currency(CURRENCY_MGP, amount)

    player:modify_crystals(CRYSTAL_ICE_SHARD, amount)

    printf(player, "Monies given!", amount)
end
