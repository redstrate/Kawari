-- Gungi Zelungi, and any other CC Attendants

-- Scenes
SCENE_00000 = 00000 -- Initial greeting

function onTalk(target, player)
    player:play_scene(target, SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    -- Opens help menu
    local target_event_id = results[2]
    player:start_event(player.id, target_event_id, EVENT_TYPE_NEST, 0)
    player:play_scene(player.id, 0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
end
