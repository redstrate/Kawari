-- Wandering Minstrel in Mor Dhona

-- Scenes
SCENE_00000 = 00000 -- Something festival related
SCENE_00001 = 00001 -- Quest-dependent cutscene

function onTalk(target, player)
    player:play_scene(target, SCENE_00001, HIDE_HOTBAR | SET_BASE, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
