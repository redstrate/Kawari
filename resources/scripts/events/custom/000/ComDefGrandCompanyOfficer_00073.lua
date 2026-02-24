-- Unknown object/NPC

-- Scenes
SCENE_00000 = 00000 -- You have to progress further in MSQ
SCENE_00001 = 00001 -- Regular menu
SCENE_00002 = 00002 -- Your present rank is:

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
