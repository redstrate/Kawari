-- Generic handler for Warps
-- Use this for most warps that are just a yes/no option

function onTalk(target, player)
    player:play_scene(00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()

    if results[1] == 1 then
        -- get warp
        player:warp(EVENT_ID)
    end
end
