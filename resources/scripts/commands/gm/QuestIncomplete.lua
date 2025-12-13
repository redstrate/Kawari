required_rank = GM_RANK_DEBUG
command_sender = "[questincomplete] "

function onCommand(args, player)
    local id <const> = args[1]
    local id2 <const> = args[2]

    -- means "all"
    if id2 == 65535 then
        player:incomplete_quest(id2)
        printf(player, "All quests incompleted!", id)
    else
        player:incomplete_quest(id)
        printf(player, "Quest "..id.." incompleted!", id)
    end
end
