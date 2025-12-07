-- Collectable Appraiser in Mor Dhona

-- Scene 0: Show menu

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
