required_rank = GM_RANK_DEBUG
command_sender = "[icon] "

function onCommand(args, player)
    local id <const> = args[1]
    player:set_online_status(id)
end
