-- Sanctum Acolyte in East Shroud

-- Scenes
SCENE_00000 = 00000 -- Greeting and wedding explanation
SCENE_00001 = 00001 -- Wristlet discard selection
SCENE_00002 = 00002 -- Wristlet discard message
SCENE_00003 = 00003 -- Error occured while discarding wristlet
SCENE_00004 = 00004 -- Ceremony preparation menu
SCENE_00005 = 00005 -- Unknown (doesn't play)

function onTalk(target, player)
    player:play_scene(target, SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
