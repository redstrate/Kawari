-- Softknox in Wolves Den Pier

-- Scenes
SCENE_00000 = 00000 -- Initial greeting

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR,{0})
end

function onYield(scene, results, player)
    -- Opens help menu
    local target_event_id = results[2]
    player:start_event(player.id, target_event_id, EVENT_TYPE_NEST, 0)
    player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
end
