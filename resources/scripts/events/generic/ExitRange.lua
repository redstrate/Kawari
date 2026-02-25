-- Generic handler for ExitRange events
-- I think this is only used for housing right now?

function onReturn(scene, results, player)
    player:finish_event()
end

function onYield(scene, id, results, player)
    local territory_id = results[3]
    local pop_range_id

    -- There's no link between these, so we have to maintain our own.
    if territory_id == TERRITORY_S1H1 then
        pop_range_id = HOUSING_POP_RANGE_S1H1
    elseif territory_id == TERRITORY_F1H1 then
        pop_range_id = HOUSING_POP_RANGE_F1H1
    elseif territory_id == TERRITORY_W1H1 then
        pop_range_id = HOUSING_POP_RANGE_W1H1
    elseif territory_id == TERRITORY_E1H1 then
        pop_range_id = HOUSING_POP_RANGE_E1H1
    elseif territory_id == TERRITORY_R1H1 then
        pop_range_id = HOUSING_POP_RANGE_R1H1
    else
        print("Unknown housing territory id: "..territory_id)
    end

    if results[1] == 1 then
        -- go to ward
        local ward_number = results[2]
        local territory_id = results[3]

        player:change_territory_pop_range(territory_id, pop_range_id)
    end
    player:finish_event()
end
