required_rank = GM_RANK_DEBUG
command_sender = "[icon] "

function onCommand(player, args, name)
    local id <const> = args[1]
    player:set_online_status(id)
end
