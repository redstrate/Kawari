required_rank = GM_RANK_DEBUG
command_sender = "[gc] "

function onCommand(args, player)
    local company = tonumber(args[1])

    if company ~= nil and company >= 0 and company <= 3 then
        player:set_grand_company(company)
        printf(player, "Grand Company set to %s.", company)
    else
        printf(player, "Invalid grand company id. Usage: //gm gc <company id> (0 = None, 1 = Maelstrom, 2 = Twin Adders, 3 = Immortal Flames)")
    end
end
