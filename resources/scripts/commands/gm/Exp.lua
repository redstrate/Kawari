required_rank = GM_RANK_DEBUG
command_sender = "[exp] "

function onCommand(args, player)
    local amount = args[1]

    player:add_exp(amount)
    printf(player, "Added %s exp.", amount)
end
