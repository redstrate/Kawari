-- Quest: Coming to Gridania
-- NOTE: These openings are so similar, ensure changes are synced between all of them!

-- scene 0: show quest prompt
-- scene 3: decline
-- scene 1: after quest acceptance
-- scene 2: you have begun a quest!
-- scene 4: miounne's cutscene
-- scene 5: accept reward dialog

ENPC_BERTENNANT = 1985150
ENPC_MIOUNNE = 1985113

function onTalk(target, player)
    if target.object_id == ENPC_BERTENNANT then
        player:play_scene(00000, HIDE_HOTBAR, {})
    elseif target.object_id == ENPC_MIOUNNE then
        player:play_scene(00004, SET_BASE | HIDE_HOTBAR, {})
    end
end

function onYield(scene, results, player)
    if scene == 4 then
        player:play_scene(00005, HIDE_HOTBAR, {})
        return
    end

    player:finish_event()
end

function onReturn(scene, results, player)
    if scene == 0 then
        -- first param: whether the quest was accepted
        local accepted = results[1] == 1
        if accepted then
            player:play_scene(00001, SET_BASE | HIDE_HOTBAR | DISABLE_SKIP, {})
            return
        else
            player:play_scene(00003, HIDE_HOTBAR, {})
            return
        end
    elseif scene == 1 then
        player:play_scene(00002, HIDE_HOTBAR, {})
        return
    elseif scene == 2 then
        player:accept_quest(EVENT_ID)

        -- call back into the opening, presumably to update the borders of the play area
        player:start_event(OPENING_EVENT_HANDLER, EVENT_TYPE_NEST, 0)
        player:play_scene(30, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {2})
        return
    elseif scene == 5 then
        local completed = results[1] == 1
        player:finish_quest(EVENT_ID)
    end

    player:finish_event()
end
