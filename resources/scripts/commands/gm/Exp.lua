required_rank = GM_RANK_DEBUG
command_sender = "[exp] "

function onCommand(player, args, name)
    local amount = args[1]

    player:add_exp(amount)
    printf(player, "Added %s exp.", amount)
end
