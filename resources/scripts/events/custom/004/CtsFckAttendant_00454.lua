-- Kasumi in Gold Saucer

-- Scene 0: Help menu

function onTalk(target, player)
    player:play_scene(target, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
