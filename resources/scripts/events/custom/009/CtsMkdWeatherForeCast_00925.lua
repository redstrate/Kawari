-- Expedition Skywatcher at the Phantom Village

-- Scenes
SCENE_MENU = 00000

function onTalk(target, player)
    player:play_scene(SCENE_MENU, 0, {0})
end

function onReturn(scene, results, player)
    player:finish_event()
end
