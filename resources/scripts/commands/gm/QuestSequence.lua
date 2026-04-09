required_rank = GM_RANK_DEBUG
command_sender = "[questsequence] "

function onCommand(args, player)
    local id <const> = args[1]
    local sequence <const> = args[2]

    player:quest_sequence(65536 + id, sequence)
    printf(player, "Set sequence in Quest "..id.." to "..sequence.."!", id)
end
