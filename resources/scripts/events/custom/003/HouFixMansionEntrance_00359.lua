-- Apartment Building Entrance, in front of the apartments of every housing ward, and in apartment lobbies, the two doors in the back next to the shop NPCs

-- TODO
-- Change param 0 in OnYield's resume_event as necessary once we support apartments.
-- Get rid of the WARD_LOBBIES hack once we figure out the proper way to move players to the lobbies
-- Hide the enter lobby menu option via however that's done, as it makes no sense to show an enter lobby option within a lobby.

-- Scenes
SCENE_SHOW_MENU = 00000 -- Shows the main menu, which offers to display the list of apartments, or to enter the lobby.

function onTalk(target, player)
    player:play_scene(SCENE_SHOW_MENU, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    local ENTER_LOBBY <const> = 1 -- The player chose to enter the apartment building lobby.
    local CANCEL_MENU <const> = -1 -- This is here for documentation purposes, no need to actually use it.
    local ENTER_MY_APARTMENT <const> = 2 -- The player chose to enter their own apartment.

    -- TODO: Is there a mapping anywhere in Excel sheets that tell us how these link up? Initial findings seem to indicate no.
    local WARD_LOBBIES <const> = {
        [339] = 573, -- Mist to Topmast Apartment Lobby,                             s1i6
        [340] = 574, -- The Lavender Beds to Lily Hills Apartment Lobby,             f1i6
        [341] = 575, -- The Goblet to Sultana's Breath Apartment Lobby,              w1i6
        [614] = 654, -- Shirogane to Kobai Goten Apartment Lobby,                    e1i6
        [979] = 985, -- Empyreum to Ingleside Apartment Lobby,                       r1i6
    }

    -- The apartment list menu isn't handled here, the client sends a CT to ask for it to be populated.
    if results[1] == ENTER_LOBBY then
        player:finish_event()
        local destination_zone = WARD_LOBBIES[player.zone.id]
        if destination_zone ~= nil then
           player:change_territory(destination_zone, { x = -0.25, y = -0.39, z = 10}, 180.0) -- TODO: Are there popranges for this anywhere? Initial findings seem to indicate no. For now we use a position captured from retail.
           return
        else -- If it's nil we're already in a ward lobby most likely, and the option to enter a lobby from within a lobby makes no sense
            player:send_message("This option shouldn't be here and is a bug in Kawari, please select a different option.")
        end
    elseif results[1] == ENTER_MY_APARTMENT then
        player:finish_event()
        local destination_zone = 609 -- Lily Hills Apartment, for now
        player:change_territory(destination_zone, { x = 0.0, y = 0.0, z = 0.0}, 180.0) -- TODO: Are there popranges for this anywhere?
        return
    end
    player:finish_event()
end

function onYield(scene, yield_id, results, player)
    if results[1] == 1 then
        -- Param 0 to this scene indicates how many apartments are in this building, so the client knows which tabs to make available. We'll set it to 90 here as a sane default.
        -- Param 1 seems to always be 1
        player:resume_event(SCENE_SHOW_MENU, yield_id, {90, 1})
        return
    end
    player:finish_event()
end
