-- Town FATE vendors in Endwalker and later expansions

function onTalk(target, player)
    -- Get the default talk for this FATE (TODO: make this based on rank)
    local rank = 0
    local target_event_id = GAME_DATA:get_fate_default_talk(BASE_ID, rank)

    -- TODO: show specialshop if rank is high enough

    player:start_event(target_event_id, EVENT_TYPE_NEST, 0)
    player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
