required_rank = GM_RANK_DEBUG
command_sender = "[togglemount] "

function onCommand(args, player)
    local id = args[1]
    player:toggle_mount(id)
end
