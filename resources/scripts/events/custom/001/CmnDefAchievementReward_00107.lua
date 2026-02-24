-- Jonathas which distributes achivement certificates

-- TODO: how is his shop menu brought up?

-- Scenes
SCENE_00000 = 00000 -- Basic greeting
SCENE_00002 = 00002 -- Achievement certificate

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
