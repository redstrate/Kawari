required_rank = GM_RANK_DEBUG
command_sender = "[questcancel] "

function onCommand(args, player)
    local id <const> = args[1]

    player:cancel_quest(65536 + id)
    printf(player, "Quest "..id.." cancelled!", id)
end
