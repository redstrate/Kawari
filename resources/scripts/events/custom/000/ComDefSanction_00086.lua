-- scene 0: services unavailable
-- scene 1: services bestowed

function onTalk(target, player)
    player:play_scene(target, 00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
