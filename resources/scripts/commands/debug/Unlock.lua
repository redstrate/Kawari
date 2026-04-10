required_rank = GM_RANK_DEBUG
command_sender = "[unlock] "

function onCommand(player, args, name)
    local argc = #args

    local usage = "\nThis command teaches the user an action, emote, etc.\nUsage: !useaction <id/all>"

    if argc < 1 then
        printf(player, "This command requires 1 parameter."..usage)
        return
    end

    if args[1] == "all" then
        player:unlock_all()
        printf(player, "Everything is unlocked, please log in again!")
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
