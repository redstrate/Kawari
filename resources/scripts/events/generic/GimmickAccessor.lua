-- TODO: figure out how shortcuts work

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 1, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
