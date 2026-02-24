-- Unknown object/NPC

-- Scenes
SCENE_00000 = 00000 -- Initial/first greeting
SCENE_00002 = 00002 -- Regular greeting
SCENE_00003 = 00003 -- New entries added
SCENE_00004 = 00004 -- A unique talk scene I don't remember the purpose of
SCENE_00005 = 00005 -- Log completion message

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
