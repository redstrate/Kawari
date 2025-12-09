-- Field FATE vendors in Endwalker and later expansions

function onTalk(target, player)
    -- To get the FATE shop ID, we have to look it up based on the Event NPC's base ID
    local fate_shop_id = player.zone:get_npc_base_id(target.object_id)

    -- Get the default talk for this FATE (TODO: make this based on rank)
    local rank = 0
    local target_event_id = GAME_DATA:get_fate_default_talk(fate_shop_id, rank)

    -- TODO: show specialshop if rank is high enough

    player:start_event(target, target_event_id, EVENT_TYPE_NEST, 0)
    player:play_scene(target, target_event_id, 0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
