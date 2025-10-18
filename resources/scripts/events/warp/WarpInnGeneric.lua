-- generic warp shared by all inn scripts

function onTalk(target, player)
    -- greeting
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {0})

    -- doesn't have inn access
    --player:play_scene(actorId, EVENT_ID, 00002, HIDE_HOTBAR, 0)
end

function onYield(scene, results, player)
    if scene == 0 then
        -- has inn access
        player:play_scene(player.id, EVENT_ID, 00001, HIDE_HOTBAR, {0})
    else
        player:finish_event(EVENT_ID)

        if results[1] == 1 then
            -- get warp
            player:warp(EVENT_ID)
        end
    end
end
