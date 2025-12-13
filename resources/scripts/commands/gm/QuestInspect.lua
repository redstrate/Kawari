required_rank = GM_RANK_DEBUG
command_sender = "[questinspect] "

function onCommand(args, player)
    local id <const> = args[1]

    -- TODO: implement
    player:send_message("test")
end
