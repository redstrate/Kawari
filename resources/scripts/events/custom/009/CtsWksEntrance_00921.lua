-- Drivingway at Bestways Burrow

-- TODO: script this menu

-- Scenes
SCENE_00000 = 00000 -- Initial greeting

function onTalk(target, player)
    player:play_scene(target, SCENE_00000, 0, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
