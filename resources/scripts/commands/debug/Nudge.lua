-- Ported from Ioncannon's Project Meteor Server 
-- https://bitbucket.org/Ioncannon/project-meteor-server/src/develop/Data/scripts/commands/gm/nudge.lua

function send_msg(message, player)
    player:send_message(string.format("[nudge] %s", message))
end

function onCommand(args, player)
    local parts = split(args)
    local argc = table.getn(parts)
    local pos = player.position
    local angle = player.rotation + (math.pi / 2)
    local distance = 5
    local direction = 0
    local arg1 = parts[1]
    local arg2 = parts[2]
    local checkArg1 = tonumber(arg1)
    local checkArg2 = tonumber(arg2)
    local vertical = {
        ["UP"] = 1,
        ["U"] = 1,
        ["+"] = 1,
        ["ASCEND"] = 1,
        ["DOWN"] = -1,
        ["D"] = -1,
        ["-"] = -1,
        ["DESCEND"] = -1,
    }

    if argc == 1 then
        if checkArg1 then
            distance = checkArg1
        else
            send_msg("Error parsing direction! Usage: !nudge <distance> <up|u|+|ascend/down|d|-|descend>", player)
            return
        end
    end

    if argc == 2 then
        if checkArg1 and checkArg2 then
            distance = checkArg1
        elseif checkArg1 and not checkArg2 then
            distance = checkArg1
            if vertical[string.upper(arg2)] then
                direction = vertical[string.upper(arg2)]
            else
                send_msg("Error parsing direction! Usage: !nudge <distance> <up|u|+|ascend/down|d|-|descend>", player)
                return
            end
        else
            send_msg("Error parsing parameters! Usage: !nudge <distance> <up/u/+/ascend/down/d/-/descend>", player)
            return
        end
    end

    local message = string.format("Positioning forward %s yalms", distance)
    local position = { x = 0.0, y = 0.0, z = 0.0 }

    if direction == 1 then
        local py = pos.y + distance
        message = string.format("Positioning up %s yalms.", distance)
        position = { x = pos.x, y = py, z = pos.z }
    elseif direction == -1 then
        local py = pos.y - distance
        message = string.format("Positioning down %s yalms.", distance)
        position = { x = pos.x, y = py, z = pos.z }
    else
        local px = pos.x - distance * math.cos(angle)
        local pz = pos.z + distance * math.sin(angle)
        if distance < 1 then
            message = string.format("Positioning back %s yalms.", distance)
        end
        position = { x = px, y = pos.y, z = pz }
    end

    player:set_position(position, player.rotation)
    send_msg(message, player)
end
