-- Drivingway at Bestways Burrow

-- Scene 0: Initial greeting

-- TODO: script this menu

function onTalk(target, player)
    player:play_scene(target, 0, 0, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
