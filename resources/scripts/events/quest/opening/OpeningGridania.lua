-- TODO: when we get a new EXDSchema release, we can automatically pull in the client-side lua variables!
-- this could be useful for the trigger IDs, for example.

--- Scene 0 is the actual cutscene
--- Scene 1 is the starting text asking you which controls you want to use.
--- Scene 20 is the "where are you going?!" message
--- Scene 30 is for removing the initial starting borders

--- We have to hardcode the pop range, because SqEx no longer provides this is in the Opening sheet
POS_START = 1317553

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

function onEnterTrigger(player)
    player:play_scene(player.id, EVENT_ID, 20, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end
