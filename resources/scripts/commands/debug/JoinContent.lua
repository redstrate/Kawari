required_rank = GM_RANK_DEBUG
command_sender = "[cf] "

function onCommand(player, args, name)
    player:join_content(args[1])
end
