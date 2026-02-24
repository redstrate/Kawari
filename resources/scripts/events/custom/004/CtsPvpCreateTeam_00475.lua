-- Team Board in Wolves Den Pier

-- Scenes
SCENE_00000 = 00000 -- Open the UI

function onTalk(target, player)
    player:play_scene(SCENE_00000, 0, {0})
end

function onReturn(scene, results, player)
    player:finish_event()
end
