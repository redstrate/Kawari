-- scene 0: show quest prompt
-- scene 3: decline
-- scene 1: after quest acceptance
-- scene 2: you have begun a quest!

-- As seen in retail
CUTSCENE_FLAGS = NO_DEFAULT_CAMERA | FADE_OUT | INVIS_EOBJ | INVIS_BNPC | INVIS_OTHER_PC | INVIS_PARTY_PC | INVIS_PARTY_BUDDY | INVIS_GATHERING_POINT | INVIS_TREASURE | CONDITION_CUTSCENE | HIDE_UI | HIDE_HOTBAR | DISABLE_SKIP | DISABLE_STEALTH | INVIS_AOE | INVIS_ALLIANCE_PC | INVIS_ALLIANCE_BUDDY | INVIS_COMPANION

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end

function onReturn(scene, results, player)
    if scene == 0 then
        -- first param: whether the quest was accepted
        local accepted = results[1] == 1
        if accepted then
            -- FIXME: i have no idea why, but the camera is suddenly stuck here right now
            player:play_scene(player.id, EVENT_ID, 00001, CUTSCENE_FLAGS, {})
            return
        else
            player:play_scene(player.id, EVENT_ID, 00003, HIDE_HOTBAR, {})
            return
        end
    elseif scene == 1 then
        player:play_scene(player.id, EVENT_ID, 00002, HIDE_HOTBAR, {})
        return
    elseif scene == 2 then
        -- call back into the opening, presumably to update the borders of the play area
        -- FIXME: doesn't work :')
        player:start_event(player.id, 1245186, EVENT_TYPE_NEST, 0)
        player:play_scene(player.id, 1245186, 30, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {2})
        return
    end

    player:finish_event(EVENT_ID)
end
