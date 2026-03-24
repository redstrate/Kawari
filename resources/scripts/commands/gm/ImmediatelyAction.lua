required_rank = GM_RANK_DEBUG
command_sender = "[teri] "

function onCommand(args, player)
    player:remove_cooldowns()
    player:send_message("Actions no longer have cooldowns!")
end
