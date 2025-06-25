-- Ported from Ioncannon's Project Meteor Server 
-- https://bitbucket.org/Ioncannon/project-meteor-server/src/develop/Data/scripts/commands/gm/nudge.lua
required_rank = GM_RANK_DEBUG
sender = "[nudge] "

function onCommand(args, player)
    local parts = split(args)
    local argc = #parts
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
    local usage = "Moves the user in a specified direction. Defaults to 5 yalms forward.\nUsage: !nudge <distance> <up|u|+|ascend/down|d|-|descend>"

    if argc == 1 then
        if checkArg1 then
            distance = checkArg1
        else
            printf(player, "Error parsing distance!\n"..usage)
            return
        end
    end

    if argc == 2 then
        if checkArg1 and checkArg2 then           -- If both are numbers, just ignore second argument
            distance = checkArg1
        elseif checkArg1 and not checkArg2 then   -- If first is number and second is string
            distance = checkArg1
            if vertical[string.upper(arg2)] then  -- Check vertical direction on string, otherwise throw param error
                direction = vertical[string.upper(arg2)]
            else
                printf(player, "Error parsing direction! \n"..usage)
                return
            end
        else
            printf(player, "Error parsing parameters! \n"..usage)
            return
        end
    end

    local direction_str = "forward"
    local new_position = { x = pos.x, y = pos.y, z = pos.z }

    if direction == 1 then
        direction_str = "up"
        new_position.y = pos.y + distance
    elseif direction == -1 then
        direction_str = "down"
        new_position.y = pos.y - distance
    else
        if distance < 1 then
            direction_str = "back"
        end
        new_position.x = pos.x - distance * math.cos(angle)
        new_position.z = pos.z + distance * math.sin(angle)
    end

    player:set_position(new_position, player.rotation)
    printf(player, "Positioning %s %s yalms.", direction_str, distance)
end
