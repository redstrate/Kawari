required_rank = GM_RANK_DEBUG
command_sender = "[questcancel] "

function onCommand(player, args, name)
    local id <const> = args[1]

    player:cancel_quest(65536 + id)
    printf(player, "Quest "..id.." cancelled!", id)
end
