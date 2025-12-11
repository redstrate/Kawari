function onTalk(target, player)
    -- TODO: Find the correct scene number
    --player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
