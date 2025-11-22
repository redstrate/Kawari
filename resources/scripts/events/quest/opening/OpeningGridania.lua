-- NOTE: These openings are so similar, ensure changes are synced between all of them!

-- TODO: when we get a new EXDSchema release, we can automatically pull in the client-side lua variables!
-- this could be useful for the trigger IDs, for example.

-- Scene 0 is the actual cutscene
-- Scene 1 is the starting text asking you which controls you want to use.
-- Scene 20 is the "where are you going?!" message
-- Scene 30 is for removing the initial starting borders

-- We have to hardcode the pop range, because SqEx no longer provides this is in the Opening sheet.
-- For future reference, this is located in the QST_OP_ENPC_001 layer.
POS_START = 2213211

-- When walking out of the city gates
ERANGE_SEQ_1_CLOSED_1 = 2351918

function onEnterTerritory(player)
    -- Move the player into the starting position
    player:move_to_pop_range(POS_START)
end

function onYield(scene, results, player)
    -- Move into the controls text after initial cutscene
    if scene == 0 then
        player:play_scene(player.id, EVENT_ID, 1, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
        return
    end

    player:finish_event(EVENT_ID)
end

function onEnterTrigger(player, arg)
    -- Play the "where are you going?!" text when entering any trigger
    if arg == ERANGE_SEQ_1_CLOSED_1 then
        player:play_scene(player.id, EVENT_ID, 20, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
    else
        -- Deciding the different messages and NPCs are actually handled client-side!
        player:play_scene(player.id, EVENT_ID, 10, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {arg})
    end
end
