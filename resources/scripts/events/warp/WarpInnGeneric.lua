function onTalk(target, player)
    -- has inn access
    player:play_scene(target, EVENT_ID, 00001, 8192, 0)

    -- doesn't have inn access
    --player:play_scene(actorId, EVENT_ID, 00002, 8192, 0)
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)

    if results[1] == 1 then
        -- get warp
        player:warp(EVENT_ID)
    end
end
