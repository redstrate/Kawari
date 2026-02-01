-- Generic handler for InstanceExit events

function onYield(scene, results, player)
    -- If chosen to leave duty:
    if #results == 1 and results[1] == 0 then
        player:abandon_content()
    end
    player:finish_event()
end
