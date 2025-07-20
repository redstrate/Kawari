-- TODO: actually implement this menu

function onTalk(target, player)
    -- unable to hire
    player:play_scene(target, EVENT_ID, 00000, 8192, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID, 0)
end
