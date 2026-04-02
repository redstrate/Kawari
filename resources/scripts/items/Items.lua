ITEM_ACTION_TYPE_MINION = 853
ITEM_ACTION_TYPE_FANTASIA = 1326 -- this is also used elsewhere, not sure what
ITEM_ACTION_TYPE_ORCHESTRION = 25183
ITEM_ACTION_TYPE_FACEWEAR = 37312

-- This is called whenever the client tries to use an item
function dispatchItem(player, id, action_type, action_data, additional_data, is_misc)
    local has_vfx = action_data[1] ~= 0
    if action_type == ITEM_ACTION_TYPE_MINION then
        return runAction("items/Minion.lua", action_data[1])
    elseif action_type == ITEM_ACTION_TYPE_FANTASIA then
        return runAction("items/Fantasia.lua", 0)
    elseif action_type == ITEM_ACTION_TYPE_ORCHESTRION then
        return runAction("items/Orchestrion.lua", additional_data)
    elseif action_type == ITEM_ACTION_TYPE_FACEWEAR then
        return runAction("items/Facewear.lua", additional_data)
    -- Otherwise, check if our item belongs to the Seasonal Miscellany or Miscellany item categories and has a vfx or not. Examples of ones that do play vfx: DAM, peach confetti. Examples of ones that don't play vfx: Realm Reborn Red, Heavenscracker.
    -- TODO: This may not be the best way but this seems to work for now
    elseif is_misc and has_vfx then
        return runAction("items/GenericVfx.lua", action_data[1])
    elseif is_misc and not has_vfx then
        return runAction("items/GenericNoEffect.lua", 0)
    else
        player:send_message("Unhandled item type: "..action_type.." (item id: "..id..")")
    end

    return nil
end
