-- scene 0: basic greeting
-- scene 2: achievement certificate

-- TODO: how is his shop menu brought up?

function onTalk(target, player, game_data)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID, 0)
end
