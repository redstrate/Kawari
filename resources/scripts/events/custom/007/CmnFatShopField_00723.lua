-- TODO: figure out why nothing shows up?

function onTalk(target, player, game_data)
    player:play_scene(target, EVENT_ID, 0, 0, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID, 0)
end
