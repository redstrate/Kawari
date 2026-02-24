-- Generic handler for SwitchTalk events

function onTalk(target, player)
    local target_event_id = player:get_switch_talk_target(EVENT_ID)

    player:start_event(target_event_id, EVENT_TYPE_NEST, 5)
    player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
