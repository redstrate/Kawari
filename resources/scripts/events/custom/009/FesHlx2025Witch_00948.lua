-- Unknown object/NPC

-- Scenes
SCENE_00000 = 00000 -- Basic greeting and help menu

function onTalk(target, player)
    player:play_scene(SCENE_00000, 0, {0})
end

function onReturn(scene, results, player)
    player:finish_event()
end
