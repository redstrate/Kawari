-- This is normally handled client-side, however in certain cases e.g. SwitchTalk we need this scripted for... reasons.

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
