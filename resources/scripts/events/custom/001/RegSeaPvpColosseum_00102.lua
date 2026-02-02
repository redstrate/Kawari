-- Berkoeya Loetahlsyn in Wolves Den Pier

-- Scenes
SCENE_00001 = 00001 -- ???

function onTalk(target, player)
    player:play_scene(SCENE_00001, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
