-- Hawu Bajihri in East Shroud

-- Scenes
SCENE_00000 = 00000 -- Greeting and explanation
SCENE_00100 = 00100 -- Anniversary set screen message
SCENE_00101 = 00101 -- Generic error message (arg is the message)
SCENE_00150 = 00150 -- Revoked registration (?)
SCENE_00200 = 00200 -- Wedding quest accepted
SCENE_00201 = 00201 -- Another arg-based thing like 101

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
