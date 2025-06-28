required_rank = GM_RANK_DEBUG
command_sender = "[item] "

function onCommand(args, player)
    local id = args[1]

    player:add_item(id)
    printf(player, "Added %s to your inventory.", id)
end
