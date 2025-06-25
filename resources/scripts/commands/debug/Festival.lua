-- A list of festival ids can be found in Hyperborea's source tree:
-- https://github.com/kawaii/Hyperborea/blob/main/Hyperborea/festivals.yaml
required_rank = GM_RANK_DEBUG
command_sender = "[festival] "

function onCommand(args, player)
    local parts = split(args)
    local argc = #parts
    local usage = "\nUsage: !festival <id1> <id2> <id3> <id4>"

    local id1 = tonumber(parts[1]) or 0
    local id2 = tonumber(parts[2]) or 0
    local id3 = tonumber(parts[3]) or 0
    local id4 = tonumber(parts[4]) or 0

    player:set_festival(id1, id2, id3, id4)
    printf(player, "Festival(s) changed to %s, %s, %s and %s.", id1, id2, id3, id4)
end
