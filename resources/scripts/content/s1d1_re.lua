-- Satasha

function onEnterTerritory(player)
    player:play_scene(player.id, EVENT_ID, 1, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {1})
end

function onYield(scene, results, player)
    player:commence_duty(EVENT_ID)
    player:finish_event(EVENT_ID)
end
