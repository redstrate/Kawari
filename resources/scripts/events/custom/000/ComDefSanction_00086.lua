-- Unknown object/NPC

-- Scenes
SCENE_00000 = 00000 -- Services unavailable
SCENE_00001 = 00001 -- Services bestowed

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
