-- Open Tournament Official in Gold Saucer

-- Scenes
SCENE_00000 = 00000 -- Welcome message
SCENE_00001 = 00001 -- Unknown (doesn't play)
SCENE_00002 = 00002 -- Open tournament hasn't begun
SCENE_00005 = 00005 -- Unknown (doesn't play)
SCENE_00006 = 00006 -- Welcome message 2

function onTalk(target, player)
    player:play_scene(SCENE_00000, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
