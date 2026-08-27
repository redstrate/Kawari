-- Generic handler for GimmickPathMove events

CONDITION = CONDITION_OCCUPIED_IN_EVENT
DISABLE_EVENT_POS_ROLLBACK = true

-- First cannon (from the left) on the Moonfire Faire boat
function moonfireFaireFirstCannonRide(player)
    local success_destination_pop_range = 12694036
    local success_client_path = 12694040

    local failure_destination_pop_ranges = {
        11607227,
        11607228,
        11607229
    }
    local failure_client_paths = {
        11614567,
        11614568,
        11614569
    }

    local chance = math.random(1, 4)
    if chance == 4 then
        player:walk_in_event(success_client_path, 150, success_destination_pop_range)
    else
        player:walk_in_event(failure_client_paths[chance], 150, failure_destination_pop_ranges[chance])
    end
end

-- Second cannon (from the left) on the Moonfire Faire boat
function moonfireFaireSecondCannonRide(player)
    local success_destination_pop_range = 12694037
    local success_client_path = 12694041

    local failure_destination_pop_ranges = {
        11607230,
        11607232,
        11607233
    }
    local failure_client_paths = {
        11614570,
        11614571,
        11614572
    }

    local chance = math.random(1, 4)
    if chance == 4 then
        player:walk_in_event(success_client_path, 150, success_destination_pop_range)
    else
        player:walk_in_event(failure_client_paths[chance], 150, failure_destination_pop_ranges[chance])
    end
end

-- Third cannon (from the left) on the Moonfire Faire boat
function moonfireFaireThirdCannonRide(player)
    local success_destination_pop_range = 12694038
    local success_client_path = 12694042

    local failure_destination_pop_ranges = {
        11607306,
        11607307,
        11607308
    }
    local failure_client_paths = {
        11614574,
        11614575,
        11614573
    }

    local chance = math.random(1, 4)
    if chance == 4 then
        player:walk_in_event(success_client_path, 150, success_destination_pop_range)
    else
        player:walk_in_event(failure_client_paths[chance], 150, failure_destination_pop_ranges[chance])
    end
end

-- Fourth cannon (from the left) on the Moonfire Faire boat
function moonfireFaireFourthCannonRide(player)
    local success_destination_pop_range = 12694039
    local success_client_path = 12694043

    local failure_destination_pop_ranges = {
        11607309,
        11607310,
        11607311
    }
    local failure_client_paths = {
        11614576,
        11614577,
        11614578
    }

    local chance = math.random(1, 4)
    if chance == 4 then
        player:walk_in_event(success_client_path, 150, success_destination_pop_range)
    else
        player:walk_in_event(failure_client_paths[chance], 150, failure_destination_pop_ranges[chance])
    end
end

-- The teleporters between plazas (and the Arcade floors) in Solution Nine
function solutionNineTeleporter(player, id)
    -- [ID] = {ClientPath, Speed, Destination PopRange}
    local teleporter_info = {
        [1] = {10114730, 270, 10114719},
        [2] = {10114817, 270, 10114728},
        [3] = {10114878, 270, 10114869},
        [4] = {10114891, 270, 10114876},
        [5] = {10114905, 180, 10114903},
        [6] = {10114944, 180, 10114902},
    }
    player:walk_in_event(table.unpack(teleporter_info[id]))
end

-- The Moonfire Faire slides on the boat
function moonfireFaireSlide(player, id)
    -- [ID] = {ClientPath, Destination PopRange}
    local slide_info = {
        [12] = {11715757, 11607154},
        [11] = {11715752, 11607153},
        [10] = {11715746, 11607151},
        [9] = {11715736, 11607150},
        [8] = {11715734, 11607149},
        [7] = {11607142, 11607148},
    }
    player:walk_in_event(slide_info[id][1], 150, slide_info[id][2])
end

-- This is used for things like the Cannon Ride in the Moonfire Faire
function onTalk(target, player)
    local id = EVENT_ID & 0xFFFF
    if id == 13 then
        moonfireFaireFirstCannonRide(player)
    elseif id == 14 then
        moonfireFaireSecondCannonRide(player)
    elseif id == 15 then
        moonfireFaireThirdCannonRide(player)
    elseif id == 16 then
        moonfireFaireFourthCannonRide(player)
    else
        player:send_message("Unscripted talk GimmickPathMove: "..id)
    end
    player:finish_event()
end

-- This is used for things like Solution Nine Teleporters
function onEnterTrigger(player, arg)
    local id = EVENT_ID & 0xFFFF
    if id == 1 or id == 2 or id == 3 or id == 4 or id == 5 or id == 6 then
        solutionNineTeleporter(player, id)
    elseif id == 7 or id == 8 or id == 9 or id == 10 or id == 11 or id == 12 then
        moonfireFaireSlide(player, id)
    else
        player:send_message("Unscripted trigger GimmickPathMove: "..id)
    end
    player:finish_event()
end
