required_rank = GM_RANK_DEBUG
command_sender = "[unlockmount] "

function onCommand(args, player)
    local id = args[1]
    player:unlock_mount(id)
end
