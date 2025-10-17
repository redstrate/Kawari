-- scene 0: you have to progress further in MSQ
-- scene 1: regular menu
-- scene 2: your present rank is:

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
