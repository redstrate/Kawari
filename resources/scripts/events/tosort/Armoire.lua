-- Internally called CmnDefCabinet:720978

-- TODO: actually implement this menu, attempting to open the "Remove an item." softlocks for now

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, 8192, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
