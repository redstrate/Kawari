-- Wandering Minstrel in Mor Dhona

-- Scene 0: Something festival related
-- Scene 1: Quest-dependent cutscne

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 1, HIDE_HOTBAR | SET_BASE, {})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
