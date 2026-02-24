-- Tournament Recordkeeper in Gold Saucer

-- Scenes
SCENE_00000 = 00000 -- Tournament menu
SCENE_00001 = 00001 -- Tournament has come to a close
SCENE_00002 = 00002 -- Upcoming tournament
SCENE_00003 = 00003 -- Hasn't completed the tutorial
SCENE_00004 = 00004 -- Completed tournament, review final standings at card square

function onTalk(target, player)
    player:play_scene(SCENE_00003, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
