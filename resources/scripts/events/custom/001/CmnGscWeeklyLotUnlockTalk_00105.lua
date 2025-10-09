-- scene 0: basic greeting

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID, 0)
end
