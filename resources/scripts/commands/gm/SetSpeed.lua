required_rank = GM_RANK_DEBUG
command_sender = "[setspeed] "

function onCommand(args, player)
    local SPEED_MAX = 10 -- Arbitrary, but it's more or less unplayable even at this amount
    local speed_multiplier = args[1]

    if speed_multiplier <= 0 then
        speed_multiplier = 1
    elseif speed_multiplier > SPEED_MAX then
        speed_multiplier = SPEED_MAX
    end

    player:set_speed(speed_multiplier)
    printf(player, "Speed multiplier set to %s.", speed_multiplier)
end
