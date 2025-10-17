-- scene 0: initial greeting, and then nothing else?

-- TODO: we probably need to see what retail does, probably event nesting

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
