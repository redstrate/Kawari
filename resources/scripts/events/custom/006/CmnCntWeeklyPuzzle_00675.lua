-- Faux Commander in Idyllshire

-- Scenes
SCENE_00000 = 00000 -- Initial greeting
SCENE_00001 = 00001 -- Menu
SCENE_00002 = 00002 -- Unknown (doesn't play)
SCENE_00003 = 00003 -- You must bring me a worthy tale
SCENE_00004 = 00004 -- Faux Hollows access denied
SCENE_00006 = 00006 -- Unknown (doesn't play)
SCENE_01001 = 01001 -- Unknown (doesn't play)

function onTalk(target, player)
    player:play_scene(target, SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
