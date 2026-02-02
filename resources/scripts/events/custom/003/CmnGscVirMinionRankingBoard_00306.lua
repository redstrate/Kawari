-- (Verminion) Tournament Ranking Board in Gold Saucer

-- Scenes
SCENE_00000 = 00000 -- Show ranking

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
