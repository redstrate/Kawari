-- Tournament Recordkeeper in Gold Saucer

-- Scene 0: Tournament menu
-- Scene 1: Tournament has come to a close
-- Scene 2: Upcoming tournament
-- Scene 3: Hasn't completed the tutorial
-- Scene 4: Completed tournament, review final standings at card square

function onTalk(target, player)
    player:play_scene(target, 3, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
