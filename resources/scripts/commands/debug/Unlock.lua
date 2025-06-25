required_rank = GM_RANK_DEBUG
sender = "[unlockaction] "

function onCommand(args, player)
    local parts = split(args)
    local argc = #parts

    local usage = "\nThis command teaches the user an action, emote, etc.\nUsage: !useaction <id/all>"

    if argc < 1 then
        printf(player, "This command requires 1 parameter."..usage)
        return
    end

    if parts[1] == "all" then
        for i = 0, 1000, 1 do
            player:unlock_action(i)
        end
        printf(player, "Everything is unlocked!", id)
    else
        local id = tonumber(parts[1])

        if not id then
            printf(player, "Error parsing action id! Make sure the id is an integer."..usage)
            return
        end

        player:unlock_action(id)
        printf(player, "Action %s unlocked!", id)
    end
end
