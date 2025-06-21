required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    local argc = table.getn(parts)
    local sender = "[unlockaetheryte] "
    local usage = "\nThis command unlocks an aetheryte for the user.\nUsage: !unlockaetheryte <id>"

    local on = tonumber(parts[1])
    local id = tonumber(parts[2])

    player:unlock_aetheryte(on, id)
    player:send_message(string.format("%s Aetheryte(s) %s had their unlocked status changed!", sender, id))
end
