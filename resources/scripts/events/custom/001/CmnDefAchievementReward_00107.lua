-- scene 0: basic greeting
-- scene 2: achievement certificate

-- TODO: how is his shop menu brought up?

function onTalk(target, player)
    player:play_scene(target, 00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
