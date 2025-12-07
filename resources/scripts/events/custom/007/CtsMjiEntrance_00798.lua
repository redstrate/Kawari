-- Baldin at Moraby Drydocks

-- Scene 0: Initial greeting
-- Scene 10: Menu
-- Scene 20: Acquaintance selection
-- Scene 30: Help menu

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, 0, {0})
end

function onYield(scene, results, player)
    if scene == 0 then
        if results[1] == 2 then
            player:change_territory(1055)
        elseif results[1] == 3 then
            -- Open selection menu
            player:play_scene(player.id, EVENT_ID, 20, 0, {0})
            return
        elseif results[1] == 5 then
            -- Open help menu
            player:play_scene(player.id, EVENT_ID, 30, 0, {0})
            return
        end
    end

    player:finish_event(EVENT_ID)
end
