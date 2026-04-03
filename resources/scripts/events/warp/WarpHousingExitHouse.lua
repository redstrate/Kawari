-- Generic warp shared by all personal houses

-- TODO
-- Ensure the player returns to a housing ward safely. For now we are unable to do this, so we send them back to the corresponding city-state's aetheryte plaza. See below for futher details.
-- Why do the Ishgard and Ul'dah warp_aetheryte calls spawn us away from the plaza and underground respectively?

-- Scenes
SCENE_EXIT_PROMPT = 00000 -- "Leave the estate hall?" prompt

function onTalk(target, player)
    player:play_scene(SCENE_EXIT_PROMPT, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    local LEAVE_HOUSE <const> = 1
    if results[1] == LEAVE_HOUSE then
        -- TODO: This is a temporary hack to ensure the player returns to a town aetheryte plaza safely after exiting a house. It's incredibly inaccurate, but at the moment we can't load housing wards from a housing interior exit, so this is the next best solution.
        local INTERIORS_TO_TOWN <const> = {
            -- Mist cottage, house, mansion to Limsa Lominsa Lower Decks
            [282] = 8,
            [283] = 8,
            [284] = 8,

            -- The Lavender Beds cottage, house, mansion to New Gridania
            [342] = 2,
            [343] = 2,
            [344] = 2,

            -- The Goblet cottage, house, mansion to Ul'dah - Steps of Nald
            [345] = 9,
            [346] = 9,
            [347] = 9,

            -- Shirogane cottage, house, mansion to Kugane
            [649] = 111,
            [650] = 111,
            [651] = 111,

            -- Empyreum cottage, house, mansion to Foundation - Ishgard
            [980] = 70,
            [981] = 70,
            [982] = 70,
        }

        -- Finish the event and decide where to go
        player:finish_event()
        player:send_message("Due to housing being in extremely early stages, you will now be moved to the nearest city-state instead of the relevant ward.")

        local destination = INTERIORS_TO_TOWN[player.zone.id]
        if destination ~= nil then
            player:warp_aetheryte(destination)
            return
        else
            player:warp_aetheryte(2) -- Fallback to New Gridania if something went wrong
            return
        end
    end

    player:finish_event()
end
