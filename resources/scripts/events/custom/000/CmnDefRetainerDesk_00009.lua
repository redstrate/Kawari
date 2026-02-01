-- Retainer management NPCs

-- Scenes
SCENE_00000 = 00000 -- Regular menu
SCENE_00001 = 00001 -- Hire a new retainer
SCENE_00002 = 00002 -- Release a retainer
SCENE_00003 = 00003 -- You cannot hire a retainer

function onTalk(target, player)
    player:play_scene(target, SCENE_00003, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
