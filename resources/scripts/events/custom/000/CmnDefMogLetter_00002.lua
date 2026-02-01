-- Unknown object/NPC

-- Scenes
SCENE_00000 = 00000 -- Unknown

function onTalk(target, player)
    player:play_scene(target, SCENE_00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
