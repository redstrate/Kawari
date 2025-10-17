-- TODO: check the quest requirement for these

function onTalk(target, player)
    local target_event_id = GAME_DATA:get_switch_talk_target(EVENT_ID)

    player:start_event(target, target_event_id, EVENT_TYPE_NEST, 5)
    player:play_scene(target, target_event_id, 0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
