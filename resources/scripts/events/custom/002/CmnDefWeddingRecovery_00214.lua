-- Sanctum Acolyte in East Shroud

-- Scene 0: Greeting and wedding explanation
-- Scene 1: Wristlet discard selection
-- Scene 2: Wristlet discard message
-- Scene 3: Error occured while discarding wristlet
-- Scene 4: Ceremony preparation menu
-- Scene 5: Unknown (doesn't play)

function onTalk(target, player)
    player:play_scene(target, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
