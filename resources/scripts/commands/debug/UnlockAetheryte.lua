required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    local argc = table.getn(parts)
    local sender = "[unlockaetheryte] "
    local usage = "\nThis command unlocks an aetheryte for the user.\nUsage: !unlockaetheryte <on/off> <id>"

    if argc < 2 then
        player:send_message(sender.."This command requires two parameters."..usage)
        return
    end

    local on = parts[1]

    if on == "on" then
        on = 0
    elseif on == "off" then
        on = 1
    else
        player:send_message(sender.."Error parsing first parameter. Must be either of the words: 'on' or 'off'."..usage)
        return
    end

    local id = tonumber(parts[2])

    if not id then
        id = parts[2]
        if id == "all" then
            id = 0
        else
            player:send_message(sender.."Error parsing id parameter. Must be a territory id or the word 'all'."..usage)
            return
        end
    end

    player:unlock_aetheryte(on, id)
    player:send_message(string.format("%s Aetheryte(s) %s had their unlocked status changed!", sender, id))
end
