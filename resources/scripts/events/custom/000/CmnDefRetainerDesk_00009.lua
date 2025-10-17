-- scene 0: regular menu
-- scene 1: hire a new retainer
-- scene 2: release a retainer
-- scene 3: you cannot hire a retainer

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00003, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
