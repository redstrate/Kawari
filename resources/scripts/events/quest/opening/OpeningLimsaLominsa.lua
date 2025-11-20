-- NOTE: These openings are so similar, ensure changes are synced between all of them!

-- Scene 0 is the actual cutscene
-- Scene 1 is the starting text asking you which controls you want to use.
-- Scene 20 is the "where are you going?!" message
-- Scene 30 is for removing the initial starting borders

-- We have to hardcode the pop range, because SqEx no longer provides this is in the Opening sheet.
-- For future reference, this is located in the EVT_OP_ONLY_BOX layer.
POS_START = 4101800

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

function onEnterTrigger(player)
    -- Play the "where are you going?!" text when entering any trigger
    player:play_scene(player.id, EVENT_ID, 20, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end
