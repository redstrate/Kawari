-- generic warp, use this for most warps that are just a yes/no option

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, 8192, 0)
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)

    if results[1] == 1 then
        -- get warp
    player:warp(EVENT_ID)
    end
end
