required_rank = GM_RANK_DEBUG
command_sender = "[item] "

function onCommand(args, player)
    local id <const> = args[1]
    local quantity = args[2]
    if quantity == 0 then
        quantity = 1
    elseif quantity > 999 then -- TODO: get the actual stack size once Lua can query item info
        quantity = 999
    end

    player:add_item(id, quantity)
    printf(player, "Added %s of item id %s to your inventory.", quantity, id)
end
