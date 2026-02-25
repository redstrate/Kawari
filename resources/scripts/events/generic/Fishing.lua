-- Generic handler for Fishing events

function onReturn(scene, results, player)
    -- Putting away fishing rod
    if scene == 3 then
        player:finish_event()
    end
end
