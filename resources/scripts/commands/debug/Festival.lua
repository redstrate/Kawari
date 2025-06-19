-- A list of festival ids can be found in Hyperborea's source tree:
-- https://github.com/kawaii/Hyperborea/blob/main/Hyperborea/festivals.yaml

function onCommand(args, player)
    local parts = split(args)
    local argc = table.getn(parts)
    local usage = "\nUsage: !festival <id> <arg1> <arg2> <arg3>"
    local sender = "[festival] "

    local arg1 = tonumber(parts[1])
    local arg2 = tonumber(parts[2])
    local arg3 = tonumber(parts[3])
    local arg4 = tonumber(parts[4])

    if not arg1 then
        player:send_message(sender.."At least one festival must be specified (for now, until the server has support for commands with no args)."..usage)
        return
    end

    if not arg2 then
        arg2 = 0
    end

    if not arg3 then
        arg3 = 0
    end

    if not arg4 then
        arg4 = 0
    end

    player:set_festival(arg1, arg2, arg3, arg4)
end
