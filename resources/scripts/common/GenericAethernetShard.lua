-- generic aetheryte, use this for all of the aethernet shards

--- scene 02 - aetheryte menu
--- scene 03 - "you have aethernet access" message and vfx

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00002, 8192, 0)
end

function onReturn(scene, results, player)
    local AETHERNET_MENU_CANCEL = 0
    local destination = results[1]
    player:finish_event(EVENT_ID)
    
    if destination ~= AETHERNET_MENU_CANCEL then
        player:warp_aetheryte(destination)
    end
end
