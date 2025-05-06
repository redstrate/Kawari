--- TODO: find a way to hardcode it this way
EVENT_ID = 131079

function onTalk(target, player)
    -- has inn access
    player:play_scene(target, EVENT_ID, 00001, 8192, 0)

    -- doesn't have inn access
    --player:play_scene(actorId, EVENT_ID, 00002, 8192, 0)
end

function onReturn(results, player)
    if results[1] == 1 then
        -- get warp
        player:warp(EVENT_ID)
    end
end
