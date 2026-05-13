-- Wameme in Old Sharlayan

-- Scenes
SCENE_GREETING = 00000

function onTalk(target, player)
    player:play_scene(SCENE_GREETING, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
