-- Hakeel Ja in Tulliyollal

-- Scene 0: Something festival related
-- Scene 1: Quest-dependent cutscne

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
