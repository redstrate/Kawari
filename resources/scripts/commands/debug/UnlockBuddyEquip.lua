required_rank = GM_RANK_DEBUG
command_sender = "[unlockbuddyequip] "

function onCommand(args, player)
    local id = args[1]

    if id == "all" then
        player:unlock_buddy_equip_all()
    else
        player:unlock_buddy_equip(tonumber(id))
    end
end
