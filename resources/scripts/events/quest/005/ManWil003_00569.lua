-- Quest: Close to Home (Ul'dah), for Pugilists
-- NOTE: These quests are so similar, ensure changes are synced between all of them!

-- scene 0: show quest prompt
-- scene 2: attunement complete cutscene
-- scene 3: welcome to the pugilists' guild
-- scene 4: marketplace dialogue
-- scene 5: you stand in the sapphire avenue exchange cutscene
-- scene 8: quest completion prompt
-- scene 50: you need to do some basic tasks

local originating_npc

function onTalk(target, player)
    originating_npc = target

    -- Show the quest prompt
    player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    printf(player, "this was scene %d", scene)
    if scene == 50 then
        -- Accept the quest, this also matches up with the client-side UI
        player:accept_quest(EVENT_ID)

        local old_position = player.position
        local old_rotation = player.rotation

        -- Just like in retail, "seamlessly" transition them to the real zone:
        player:change_territory(130, { x = old_position.x, y = old_position.y, z = old_position.z }, old_rotation)
    end

    player:finish_event(EVENT_ID)
end

function onReturn(scene, results, player)
    if scene == 0 and results[1] == 1 then
        -- Play the introductory text if accepted (this has to be played from Momodi)
        player:play_scene(originating_npc, EVENT_ID, 50, HIDE_HOTBAR, {})
        return
    end

    player:finish_event(EVENT_ID)
end
