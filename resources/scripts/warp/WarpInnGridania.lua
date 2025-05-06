--- TODO: find a way to hardcode it this way
EVENT_ID = 131079 -- TODO: wrong, i was testing in limsa

function onTalk(actorId, player)
    -- has inn access
    -- player:play_scene(131079, 00001, 1, 0)

    -- doesn't have inn access
    player:play_scene(actorId, EVENT_ID, 00002, 8192, 0)
end
