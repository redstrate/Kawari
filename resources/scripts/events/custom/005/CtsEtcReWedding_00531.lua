-- Hawu Bajihri in East Shroud

-- Scene 0: Greeting and explanation
-- Scene 100: Anniversary set screen message
-- Scene 101: Generic error message (arg is the message)
-- Scene 150: Revoked registration (?)
-- Scene 200: Wedding quest accepted
-- Scene 201: Another arg-based thing like 101

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
