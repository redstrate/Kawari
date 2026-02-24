-- Foreign Levy Flyer, and any other Frontline Attendants

-- Scenes
SCENE_00000 = 00000 -- Initial/first greeting

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    -- Opens help menu
    local target_event_id = results[2]
    player:start_event(target_event_id, EVENT_TYPE_NEST, 0)
    player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
end
