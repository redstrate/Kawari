function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
