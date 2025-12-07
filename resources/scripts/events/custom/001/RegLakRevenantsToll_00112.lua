-- Alison in Mor Dhona

-- Scene 0: Initial greeting
-- Scene 1: Help menu

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 then
        -- Show help menu
        player:play_scene(player.id, EVENT_ID, 1, HIDE_HOTBAR, {})
        return
    end
    player:finish_event(EVENT_ID)
end
