-- Hakeel Ja in Tulliyollal

-- Scenes
SCENE_00000 = 00000 -- Something festival related
SCENE_00001 = 00001 -- Quest-dependent cutscne

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
