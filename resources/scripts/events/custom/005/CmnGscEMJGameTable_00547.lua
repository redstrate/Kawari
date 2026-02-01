-- Mahjong Tables in Gold Saucer

-- Scene 0: Start Mahjong Solo

function onTalk(target, player)
    player:play_scene(target, 0, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
