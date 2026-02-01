-- scene 0: initial/first greeting
-- scene 2: regular greeting
-- scene 3: new entries added
-- scene 4: a unique talk scene I don't remember the purpose of
-- scene 5: log completion message

function onTalk(target, player)
    player:play_scene(target, 00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
