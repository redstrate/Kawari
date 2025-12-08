-- Open Tournament Official in Gold Saucer

-- Scene 0: Welcome message
-- Scene 1: Unknown (doesn't play)
-- Scene 2: Open tournament hasn't begun
-- Scene 5: Unknown (doesn't play)
-- Scene 6: Welcome message 2

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
