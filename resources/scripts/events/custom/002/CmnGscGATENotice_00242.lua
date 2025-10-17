-- scene 0: event is currently being held
-- scene 1: currently crashes the client?
-- scene 2: next event will happen

function onTalk(target, player)
    -- first arg is the GATE, second arg is the location
    -- currently placeholders for now
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {5, 1})
end

function onReturn(scene, results, player)
    -- first result is 1 if requesting to warp, otherwise 0
    player:finish_event(EVENT_ID)
end
