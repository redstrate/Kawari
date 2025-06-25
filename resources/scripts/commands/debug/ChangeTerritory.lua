required_rank = GM_RANK_DEBUG
sender = "[teri] "

function onCommand(args, player)
    local parts = split(args)
    local argc = #parts
    local usage = "\nThis command moves the user to a new zone/territory.\nUsage: !teri <id>"

    if argc == 0 then
        printf(player, "A territory id is required to use this command."..usage)
        return
    end

    local id = tonumber(parts[1])

    if not id or id < 0 or id > 65535 then -- Must be in range of unsigned 16-bit value
        printf(player, "Error parsing territory id! Make sure your input is an integer between 0 and 65535."..usage)
        return
    end

    player:change_territory(id)
    printf(player, "Changing territory to %s.", id)
end
