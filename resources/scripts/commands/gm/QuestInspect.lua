required_rank = GM_RANK_DEBUG
command_sender = "[questinspect] "

function onCommand(player, args, name)
    local id <const> = args[1]

    -- TODO: implement
    player:send_message("test")
end
