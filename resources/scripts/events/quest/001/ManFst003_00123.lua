-- Quest: Close to Home (Gridania), for Archers
-- NOTE: These quests are so similar, ensure changes are synced between all of them!

-- scene 0: show quest prompt

function onTalk(target, player)
    -- Show the quest prompt
    player:play_scene(0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 50 then
        -- Accept the quest, this also matches up with the client-side UI
        player:accept_quest(EVENT_ID)

        local old_position = player.position
        local old_rotation = player.rotation

        -- Just like in retail, "seamlessly" transition them to the real zone:
        player:change_territory(TERRITORYTYPE0, { x = old_position.x, y = old_position.y, z = old_position.z }, old_rotation)
    end

    player:finish_event()
end

function onReturn(scene, results, player)
    if scene == 0 and results[1] == 1 then
        -- Play the introductory text if accepted (this has to be played from Momodi)
        player:play_scene(50, HIDE_HOTBAR, {})
        return
    end

    player:finish_event()
end
