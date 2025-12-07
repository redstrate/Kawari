-- Disreputable Priest in Wolves Den Pier

-- Scene 0: Open shop menu
-- Scene 1: Acquired all titles
-- Scene 2: Some other title acquisition response
-- Scene 3: Acquiring a new title
-- Scene 100: "That's not what I'm looking for" message

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    -- TODO: implement the shop

    player:finish_event(EVENT_ID)
end
