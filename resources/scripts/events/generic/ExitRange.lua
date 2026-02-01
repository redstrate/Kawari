-- Generic handler for ExitRange events

function onYield(scene, results, player)
    player:finish_event()
end

function onReturn(scene, results, player)
    if results[1] == 1 then
        -- go to ward
        local ward_number = results[2]
        local territory_id = results[3]

        player:change_territory(territory_id)
    end
    player:finish_event()
end
