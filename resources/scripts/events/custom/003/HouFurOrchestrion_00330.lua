-- Scene 00000 opens the main player
-- Scene 00001 opens the playlist editor, but right now, closing it softlocks, and trying to edit anything says you are not authorized to use the estate's orchestrion

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID, 0)
end
