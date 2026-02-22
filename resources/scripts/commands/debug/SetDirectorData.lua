required_rank = GM_RANK_DEBUG
command_sender = "[setdirectordata] "

function onCommand(args, player)
    player:set_director_data(args[1], args[2])
end
