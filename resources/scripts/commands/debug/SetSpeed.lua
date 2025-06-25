required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    local parts = split(args)
    local argc = #parts
    local sender = "[setspeed] "
    local usage = "\nThis command sets the user's speed to a desired multiplier.\nUsage: !setspeed <multiplier>"
    local SPEED_MAX = 10 -- Arbitrary, but it's more or less unplayable even at this amount
    local speed_multiplier = tonumber(parts[1])

    if argc == 1 and not speed_multiplier then
        player:send_message(sender.."Error parsing speed multiplier! Make sure the multiplier is an integer."..usage)
        return
    elseif argc == 0 then
        speed_multiplier = 1
    end

    if speed_multiplier <= 0 then
        speed_multiplier = 1
    elseif speed_multiplier > SPEED_MAX then
        speed_multiplier = SPEED_MAX
    end

    player:set_speed(speed_multiplier)
    player:send_message(string.format("%sSpeed multiplier set to %s.", sender, speed_multiplier))
end
