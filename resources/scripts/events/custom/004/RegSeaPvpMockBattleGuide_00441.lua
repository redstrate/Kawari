-- Softknox in Wolves Den Pier

function onTalk(target, player)
    player:play_scene(target, 00000, HIDE_HOTBAR,{0})
end

function onYield(scene, results, player)
    -- Opens help menu
    local target_event_id = results[2]
    player:start_event(player.id, target_event_id, EVENT_TYPE_NEST, 0)
    player:play_scene(player.id, 0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
end
