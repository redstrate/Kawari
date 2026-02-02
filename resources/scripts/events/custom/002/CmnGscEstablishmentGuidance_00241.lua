-- Gold Saucer Attendant in Gold Saucer

-- Scenes
SCENE_00000 = 00000 -- Basic greeting

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
