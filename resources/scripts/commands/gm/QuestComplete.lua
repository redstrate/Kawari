required_rank = GM_RANK_DEBUG
command_sender = "[questcomplete] "

function onCommand(args, player)
    local id <const> = args[1]
    local id2 <const> = args[2]

    -- means "all"
    if id2 == 65535 then
        player:finish_quest(id2)
        printf(player, "All quests completed!", id)
    else
        player:finish_quest(id)
        printf(player, "Quest "..id.." completed!", id)
    end
end
