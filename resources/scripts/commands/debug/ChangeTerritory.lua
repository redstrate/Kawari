required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    local argc = table.getn(parts)
    local sender = "[teri] "
    local usage = "\nThis command moves the user to a new zone/territory.\nUsage: !teri <id>"

    if argc == 0 then
        player:send_message(sender.."A territory id is required to use this command."..usage)
    end

    local id = tonumber(parts[1])

    if not id then
        player:send_message(sender.."Error parsing territory id! Make sure your input is an integer."..usage)
        return
    end

    player:change_territory(id)
    player:send_message(string.format("%s Changing territory to %s.", sender, id))
end
