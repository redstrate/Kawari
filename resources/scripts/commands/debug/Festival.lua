-- A list of festival ids can be found in Hyperborea's source tree:
-- https://github.com/kawaii/Hyperborea/blob/main/Hyperborea/festivals.yaml
required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    local argc = #parts
    local usage = "\nUsage: !festival <id1> <id2> <id3> <id4>"
    local sender = "[festival] "

    local id1 = tonumber(parts[1])
    local id2 = tonumber(parts[2])
    local id3 = tonumber(parts[3])
    local id4 = tonumber(parts[4])

    if not id1 then
        id1 = 0
    end

    if not id2 then
        id2 = 0
    end

    if not id3 then
        id3 = 0
    end

    if not id4 then
        id4 = 0
    end

    player:set_festival(id1, id2, id3, id4)
    local message = string.format("Festival(s) changed to %s, %s, %s and %s.", id1, id2, id3, id4)
    player:send_message(message)
end
