-- Internally called CmnDefMiniGame:721096

-- TODO: actually implement this menu

function onTalk(target, player)
    -- You have not yet unlocked any mini-games.
    player:play_scene(target, EVENT_ID, 00000, 8192, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID, 0)
end
