-- NOTE: These openings are so similar, ensure changes are synced between all of them!

-- Scene 0 is the actual cutscene
-- Scene 1 is the starting text asking you which controls you want to use.

-- We have to hardcode the pop range, because SqEx no longer provides this is in the Opening sheet.
-- For future reference, this is from the EVT_OP_ONLY_BOX layer.
POS_START = 4101669

-- When walking out of the city gates
ERANGE_SEQ_1_CLOSED_1 = 4101587

function onEnterTerritory(player)
    --- Move the player into the starting position
    player:move_to_pop_range(POS_START)
end

function onYield(scene, results, player)
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
