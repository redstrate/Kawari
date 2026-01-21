required_rank = GM_RANK_DEBUG
command_sender = "[cf] "

function onCommand(args, player)
    player:join_content(args[1])
end
