-- Team Board in Wolves Den Pier

-- Scene 0: Open the UI

function onTalk(target, player)
    player:play_scene(target, 0, 0, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
