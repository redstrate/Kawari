SCENE_SHOW_MENU = 00000 -- Shows the main menu, which offers to display the list of apartments, or to enter the lobby.

function onTalk(target, player)
    player:play_scene(SCENE_SHOW_MENU, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    local ENTER_LOBBY <const> = 1 -- The player chose to enter the apartment building lobby.
    local CANCEL_MENU <const> = -1 -- This is here for documentation purposes, no need to actually use it.

    -- The apartment list menu isn't handled here, the client sends a CT to ask for it to be populated.
    if results[1] == ENTER_LOBBY then
        player:finish_event()
        -- TODO: Actually warp to apartment lobbies, they hang for now for some reason
        player:send_message("The apartment lobby is currently unavailable.")
        return
    end
    player:finish_event()
end

function onYield(scene, yield_id, results, player)
    if results[1] == 1 then
        -- TODO: Change param 0 as necessary once we support apartments.
        -- Param 0 to this scene indicates how many apartments are in this building, so the client knows which tabs to make available. We'll set it to 90 here as a sane default.
        -- Param 1 seems to always be 1
        player:resume_event(SCENE_SHOW_MENU, yield_id, {90, 1})
        return
    end
    player:finish_event()
end
