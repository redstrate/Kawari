required_rank = GM_RANK_DEBUG
command_sender = "[unlock] "

function onCommand(args, player)
    local argc = #args

    local usage = "\nThis command teaches the user an action, emote, etc.\nUsage: !useaction <id/all>"

    if argc < 1 then
        printf(player, "This command requires 1 parameter."..usage)
        return
    end

    if args[1] == "all" then
        for i = 0, 511, 1 do
            player:unlock(i)
        end
        printf(player, "Everything is unlocked!", id)
    else
        local id = tonumber(args[1])

        if not id then
            printf(player, "Error parsing unlock id! Make sure the id is an integer."..usage)
            return
        end

        player:unlock(id)
        printf(player, "%s unlocked!", id)
    end
end
