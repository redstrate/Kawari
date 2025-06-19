-- A list of festival ids can be found in Hyperborea's source tree:
-- https://github.com/kawaii/Hyperborea/blob/main/Hyperborea/festivals.yaml

function onCommand(args, player)
    local parts = split(args)
    local argc = table.getn(parts)
    local usage = "\nUsage: !festival <id> <arg1> <arg2> <arg3>"
    local sender = "[festival] "
    if argc < 4 then
        player:send_message(sender.."This command requires 4 parameters."..usage)
        return
    end

    arg1 = tonumber(parts[1])
    arg2 = tonumber(parts[2])
    arg3 = tonumber(parts[3])
    arg4 = tonumber(parts[4])
    
    if not arg1 or not arg2 or not arg3 or not arg4 then
        player:send_message(sender.."Error parsing parameters. Make sure your inputs are integers."..usage)
        return
    end

    player:set_festival(arg1, arg2, arg3, arg4)
end
