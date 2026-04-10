required_rank = GM_RANK_DEBUG
command_sender = "[howto] "

function onCommand(player, args, name)
    local on <const> = ~args[1] & 1  -- The client sends 1 for off and 0 for on, so we need to invert this for the rust side to work properly.
    local id <const> = args[2]

    player:toggle_howto(on, id)
    printf(player, "Active Help(s) %s had their read status changed! You will need to log in again for this to take effect.", id)
end
