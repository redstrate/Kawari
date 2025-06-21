required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    local argc = table.getn(parts)
    local sender = "[unlockaction] "
    local usage = "\nThis command teaches the user an action.\nUsage: !useaction <id>"

    if argc < 1 then
        player:send_message(sender.."This command requires 1 parameter."..usage)
        return
    end

    local id = tonumber(parts[1])

    if not id then
        player:send_message(sender.."Error parsing action id! Make sure the id is an integer."..usage)
        return
    end

    player:unlock_action(id)
    player:send_message(sender.."Action unlocked!")
end
