-- scene 0: services unavailable
-- scene 1: services bestowed

function onTalk(target, player, game_data)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
