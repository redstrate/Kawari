-- Triple Triad Master in Gold Saucer

-- Scene 0: Initial greeting
-- Scene 1: Help menu

function onTalk(target, player)
    player:play_scene(target, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 then
        -- Show help menu
        player:play_scene(player.id, 1, HIDE_HOTBAR, {})
        return
    end
    player:finish_event()
end
