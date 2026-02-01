-- Faux Commander in Idyllshire

-- Scene 0: Initial greeting
-- Scene 1: Menu
-- Scene 2: Unknown (doesn't play)
-- Scene 3: You must bring me a worthy tale
-- Scene 4: Faux Hollows access denied
-- Scene 6: Unknown (doesn't play)
-- Scene 1001: Unknown (doesn't play)

function onTalk(target, player)
    player:play_scene(target, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
