--- TODO: find a way to hardcode it this way
EVENT_ID = 131082

-- TODO: it seems that these all might share one common function, and the only difference is the event id

function onTalk(target, player)
    --- prompt to exit the inn
    player:play_scene(target, EVENT_ID, 00000, 8192, 0)
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)

    if results[1] == 1 then
        -- get warp
        player:warp(EVENT_ID)
    end
end
