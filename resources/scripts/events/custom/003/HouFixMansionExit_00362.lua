-- Apartment Building Exit, the front door in every apartment lobby

-- TODO
-- Ensure the player returns to a housing ward safely. For now we are unable to do this, so we send them back to the corresponding city-state's aetheryte plaza. See below for futher details.
-- Why do the Ishgard and Ul'dah warp_aetheryte calls spawn us away from the plaza and underground respectively?
-- Hide the enter lobby menu option via however that's done, as it makes no sense to show an enter lobby option within a lobby.

-- Scenes
SCENE_SHOW_MENU = 00000 -- Shows the main menu, which offers to display the list of apartments, or exit the building.

function onTalk(target, player)
    player:play_scene(SCENE_SHOW_MENU, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    local EXIT_BUILDING <const> = 1
    local ENTER_LOBBY <const> = 2 -- Retail never shows this. We're currently not setting something properly to hide it.

    if results[1] == EXIT_BUILDING then
        -- TODO: This is a temporary hack to ensure the player returns to a town aetheryte plaza safely after exiting an apartment or lobby. It's incredibly inaccurate, but at the moment we can't load housing wards from a housing interior exit, so this is the next best solution.
        local INTERIORS_TO_TOWN <const> = {
            -- Apartment lobbies
            [573] = 8, -- Topmast Apartment Lobby to Limsa Lominsa Lower Decks
            [574] = 9, -- Sultana's Breath Apartment Lobby to Ul'dah - Steps of Nald
            [575] = 2, -- Lily Hills Apartment Lobby to New Gridania
            [654] = 111, -- Kobai Goten Apartment Lobby to Kugane
            [985] = 70, -- Ingleside Apartment Lobby to Foundation - Ishgard

            -- Apartment interiors
            [608] = 8, -- Topmast Apartment to Limsa Lominsa Lower Decks
            [609] = 2, -- Lily Hills Apartment to New Gridania
            [610] = 9, -- Sultana's Breath Apartment to Ul'dah - Steps of Nald
            [655] = 111, -- Kobai Goten Apartment to Kugane
            [999] = 70, -- Ingleside Apartment to Foundation - Ishgard
        }
        
        player:finish_event()
        player:send_message("Due to housing being in extremely early stages, you will now be moved to the nearest city-state instead of the relevant ward.")

        local destination = INTERIORS_TO_TOWN[player.zone.id]
        if destination ~= nil then
            player:warp_aetheryte(destination)
            return
        else
            player:warp_aetheryte(2) -- Fallback to New Griania if something went wrong
            return
        end
    elseif results[1] == ENTER_LOBBY then -- TODO: remove this once we can hide this menu option
        player:send_message("This option shouldn't be here and is a bug in Kawari, please select a different option.")
    end
    
    player:finish_event()
end

function onYield(scene, yield_id, results, player)
    if results[1] == 1 then
        player:resume_event(SCENE_SHOW_MENU, yield_id, {90, 1})
        return
    end
    player:finish_event()
end
