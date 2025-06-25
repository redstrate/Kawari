required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    local argc = #parts
    local sender = "[unlockaction] "
    local usage = "\nThis command teaches the user an action, emote, etc.\nUsage: !useaction <id/all>"

    if argc < 1 then
        player:send_message(sender.."This command requires 1 parameter."..usage)
        return
    end

    if parts[1] == "all" then
        for i = 0, 1000, 1 do
            player:unlock_action(i)
        end
        player:send_message(string.format("%s Everything is unlocked!", sender, id))
    else
        local id = tonumber(parts[1])

        if not id then
            player:send_message(sender.."Error parsing action id! Make sure the id is an integer."..usage)
            return
        end

        player:unlock_action(id)
        player:send_message(string.format("%s Action %s unlocked!", sender, id))
    end
end
