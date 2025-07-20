-- Internally called CmnDefPrismBox:721347

-- TODO: actually implement this menu
-- Cutscene 0, flags 247 ("HOW_TO_ID_2", according to Scripter) fades the screen to black for a while, then comes back after closing an invisible dialog box

function onTalk(target, player)
    -- You have not yet unlocked the glamour dresser.
    player:play_scene(target, EVENT_ID, 00000, 8192, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID, 0)
end
