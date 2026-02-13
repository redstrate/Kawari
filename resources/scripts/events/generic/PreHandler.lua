-- Generic handler for PreHandler events

-- TODO: check the quest requirement for these

function onTalk(target, player)
    local target_event_id = GAME_DATA:get_pre_handler_target(EVENT_ID)

    player:start_event(target_event_id, EVENT_TYPE_NEST, 0)

    -- NOTE: not sure if this is 100% true
    local event_type = target_event_id >> 16
    if event_type == HANDLER_TYPE_GIL_SHOP then
        player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
    elseif event_type == HANDLER_TYPE_INCLUSION_SHOP then
        player:play_scene(1, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
    elseif event_type == HANDLER_TYPE_SPECIAL_SHOP then
        player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
    elseif event_type == HANDLER_TYPE_DESCRIPTION then
        player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
    else
        print("Unknown PreHandler target event type: "..event_type)
    end
end

function onYield(scene, results, player)
    player:finish_event()
end
