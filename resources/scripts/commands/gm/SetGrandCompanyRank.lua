required_rank = GM_RANK_DEBUG
command_sender = "[gcrank] "

function onCommand(player, args, name)
    local rank = tonumber(args[1])
    local MAX_RANK <const> = 11 -- As of Dawntrail, the max rank in a GC is 11, or Captain.

    if rank ~= nil and player.active_grand_company ~= 0 and rank > 0 and rank <= MAX_RANK then
        player:set_grand_company_rank(rank)
        printf(player, "Grand Company rank set to %s.", rank)
    else
        printf(player, "Cannot set grand company rank: you must be in a grand company, and the rank you are setting must be between 1 and %s.", MAX_RANK)
    end
end
