-- Mahjong Tables in Gold Saucer

-- Scenes
SCENE_00000 = 00000 -- Start Mahjong Solo

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
