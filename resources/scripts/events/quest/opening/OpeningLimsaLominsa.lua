--- Scene 0 is the actual cutscene
--- Scene 1 is the starting text asking you which controls you want to use.

--- We have to hardcode the pop range, because SqEx no longer provides this is in the Opening sheet
POS_START = 4101800

function onEnterTerritory(player)
    --- Move the player into the starting position
    player:move_to_pop_range(POS_START)
end

function onYield(scene, results, player)
    if scene == 0 then
        player:play_scene(player.id, EVENT_ID, 1, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {0})
    elseif scene == 1 then
        -- just quit for now
        player:finish_event(EVENT_ID)
    end
end
