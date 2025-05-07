-- TODO: actually implement this menu

function onTalk(target, player)
    -- you cannot consult, which is good because we don't know how to implement this anyway
    player:play_scene(target, EVENT_ID, 00000, 8192, 0)
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
