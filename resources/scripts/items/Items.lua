ITEM_ACTION_TYPE_MINION = 853
ITEM_ACTION_TYPE_FANTASIA = 1326 -- this is also used elsewhere, not sure what
ITEM_ACTION_TYPE_ORCHESTRION = 25183

-- This is called whenever the client tries to use an item
function dispatchItem(player, game_data, id, action_type, action_data, additional_data)
    if action_type == ITEM_ACTION_TYPE_MINION then
        return runAction("items/Minion.lua", action_data[1])
    elseif action_type == ITEM_ACTION_TYPE_FANTASIA then
        return runAction("items/Fantasia.lua", 0)
    elseif action_type == ITEM_ACTION_TYPE_ORCHESTRION then
        return runAction("items/Orchestrion.lua", additional_data)
    else
        player:send_message("Unhandled item type: "..action_type.." (item id: "..id..")")
    end

    return nil
end
