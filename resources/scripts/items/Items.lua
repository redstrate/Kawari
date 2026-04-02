ITEM_ACTION_TYPE_MINION = 853
ITEM_ACTION_TYPE_FANTASIA = 1326 -- this is also used elsewhere, not sure what
ITEM_ACTION_TYPE_ORCHESTRION = 25183
ITEM_ACTION_TYPE_FACEWEAR = 37312
ITEM_ACTION_TYPE_DAM = 44290 -- This seems to be the only one of its kind for now, it's the energy drink from S9 in Dawntrail
ITEM_ACTION_TYPE_FIREWORKS = 944 -- Maybe rename this if other misc. vfx items use this action type
ITEM_ACTION_TYPE_ARR = 2645 -- A Realm Reborn wine bottle, TODO: This doesn't work yet

-- This is called whenever the client tries to use an item
function dispatchItem(player, id, action_type, action_data, additional_data)
    if action_type == ITEM_ACTION_TYPE_MINION then
        return runAction("items/Minion.lua", action_data[1])
    elseif action_type == ITEM_ACTION_TYPE_FANTASIA then
        return runAction("items/Fantasia.lua", 0)
    elseif action_type == ITEM_ACTION_TYPE_ORCHESTRION then
        return runAction("items/Orchestrion.lua", additional_data)
    elseif action_type == ITEM_ACTION_TYPE_FACEWEAR then
        return runAction("items/Facewear.lua", additional_data)
    elseif action_type == ITEM_ACTION_TYPE_DAM or ITEM_ACTION_TYPE_FIREWORKS then
        return runAction("items/GenericVfx.lua", action_data[1])
    else
        player:send_message("Unhandled item type: "..action_type.." (item id: "..id..")")
    end

    return nil
end
