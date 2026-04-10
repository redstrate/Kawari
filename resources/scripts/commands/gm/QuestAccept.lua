required_rank = GM_RANK_DEBUG
command_sender = "[questaccept] "

function onCommand(player, args, name)
    local id <const> = args[1]

    player:accept_quest(65536 + id)
    printf(player, "Quest "..id.." accepted!", id)
end
