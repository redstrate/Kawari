-- Generic handler for InstanceExit events

CONDITION = CONDITION_OCCUPIED_IN_EVENT

function onReturn(scene, results, player)
    -- If chosen to leave duty:
    if #results == 1 and results[1] == 0 then
        player:abandon_content()
    end
    player:finish_event()
end
