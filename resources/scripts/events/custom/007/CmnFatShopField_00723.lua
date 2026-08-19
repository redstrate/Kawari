-- Field FATE vendors in Endwalker and later expansions

-- DefaultTalk[0] is "story needs progress"
-- DefaultTalk[3] is "ready to trade"
-- DefaultTalk[4] is explanatory text
-- DefaultTalk[5] is rank 2
-- DefaultTalk[6] is rank 3
-- DefaultTalk[7] is rank 4
-- DefaultTalk[8] is all zones(?)

function determineDefaultTalk(rank)
    if rank == 0 then
        return 0
    elseif rank == 1 then
        return 3
    elseif rank == 2 then
        return 5
    elseif rank == 3 then
        return 6
    elseif rank == 4 then
        return 7
    end

    return nil
end

function determineSpecialShop(rank)
    if rank == 1 or rank == 2 then
        return 0
    elseif rank == 3 then
        return 1
    elseif rank == 4 then
        return 2
    end

    return nil
end

function onTalk(target, player)
    -- Get the default talk for this FATE
    local rank = player:get_territory_fate_rank()
    local target_event_id = GAME_DATA:get_fate_default_talk(BASE_ID, determineDefaultTalk(rank))

    player:start_event(target_event_id, EVENT_TYPE_NEST, 0)
    player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
end

function onReturn(scene, results, player)
    -- HACK: 255 is not a real scene, this is a Kawari implementation detail to tell we returned from nesting
    if scene == 255 then
        local rank = player:get_territory_fate_rank()
        local special_shop_id = determineSpecialShop(rank)
        if special_shop_id ~= nil then
            local target_event_id = GAME_DATA:get_fate_special_shop(BASE_ID, special_shop_id)

            player:start_event(target_event_id, EVENT_TYPE_NEST, 0)
            player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
            return
        end
    end

    player:finish_event()
end
