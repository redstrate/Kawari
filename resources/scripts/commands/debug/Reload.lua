required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    player:reload_scripts()
    player:send_message("Scripts reloaded!")
end
