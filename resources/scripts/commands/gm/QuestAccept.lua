required_rank = GM_RANK_DEBUG
command_sender = "[questaccept] "

function onCommand(args, player)
    local id <const> = args[1]

    player:accept_quest(id)
    printf(player, "Quest "..id.." accepted!", id)
end
