required_rank = 255 -- Doesn't exist, used for purposes of testing permissions in scripts

function onCommand(args, player)
    player:send_message("How did you run this? This shouldn't be runnable!")
end
