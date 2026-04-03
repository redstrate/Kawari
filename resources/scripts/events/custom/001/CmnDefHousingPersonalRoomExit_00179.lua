-- Generic room exit shared by all FC private chambers

-- TODO
-- Ensure the player returns to their free company's housing interior safely. For now we are unable to do this, so we send them back to the corresponding city-state's aetheryte plaza. See below for futher details.
-- Why do the Ishgard and Ul'dah warp_aetheryte calls spawn us away from the plaza and underground respectively?

-- Scenes
SCENE_MENU = 00000 -- Menu

function onTalk(target, player)
    player:play_scene(SCENE_MENU, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    local LEAVE_CHAMBERS <const> = 2
    if results[1] == LEAVE_CHAMBERS then
        -- TODO: This is a temporary hack to ensure the player returns to a town aetheryte plaza safely after exiting an FC private chamber. It's incredibly inaccurate, but at the moment we can't load housing wards or other housing interiors from a housing interior exit, so this is the next best solution.
        local INTERIORS_TO_TOWN <const> = {
            [384] = 8,   -- Mist private chambers to Limsa Lominsa Lower Decks
            [385] = 2,   -- The Lavender Beds private chambers to New Gridania
            [386] = 9,   -- The Goblet private chambers to Ul'dah - Steps of Nald
            [652] = 111, -- Shirogane private chambers to Kugane
            [983] = 70,  -- Foundation private chambers to Foundation - Ishgard
        }

        -- Finish the event and decide where to go
        player:finish_event()
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

function onYield(scene, yield_id, results, player)
    player:resume_event(SCENE_MENU, yield_id, {0})
end
