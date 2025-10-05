required_rank = GM_RANK_DEBUG
command_sender = "[toggleorchestrion] "

function onCommand(args, player)
    local id = args[1]
    player:toggle_orchestrion(id)
end
