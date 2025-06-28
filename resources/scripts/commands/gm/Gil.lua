required_rank = GM_RANK_DEBUG
command_sender = "[gil] "

function onCommand(args, player)
    local amount = args[1]

    player:add_gil(amount)
    printf(player, "Added %s gil.", amount)
end
