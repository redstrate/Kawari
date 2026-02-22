required_rank = GM_RANK_DEBUG
command_sender = "[setdirectordata] "

function onCommand(args, player)
    -- TODO: support setting other indices in the array
    player:set_director_data(args[1])
end
