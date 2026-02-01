-- Berkoeya Loetahlsyn in Wolves Den Pier
-- Scene 0: Initial greeting

function onTalk(target, player)
    player:play_scene(target, 1, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
