-- Quest: Coming to Limsa
-- NOTE: These openings are so similar, ensure changes are synced between all of them!

-- scene 0: show quest prompt
-- scene 3: decline
-- scene 1: after quest acceptance
-- scene 2: you have begun a quest!
-- scene 4: unk
-- scene 5: take the crow's lift
-- scene 11: cutscene with grehfarr
-- scene 12: quest completed prompt

-- As seen in retail
CUTSCENE_FLAGS = NO_DEFAULT_CAMERA | FADE_OUT | INVIS_EOBJ | INVIS_BNPC | INVIS_OTHER_PC | INVIS_PARTY_PC | INVIS_PARTY_BUDDY | INVIS_GATHERING_POINT | INVIS_TREASURE | CONDITION_CUTSCENE | HIDE_UI | HIDE_HOTBAR | DISABLE_SKIP | DISABLE_STEALTH | INVIS_AOE | INVIS_ALLIANCE_PC | INVIS_ALLIANCE_BUDDY | INVIS_COMPANION

ENPC_RYSSFLOH = 4102039
ENPC_BADERON = 4102072
ENPC_GREHFARR = 4107186

-- Destination for the Crow's Lift
-- This located in the EVT_OP_ONLY_ENPC layer.
POS_INN_WARP = 4127803

local originating_npc

function onTalk(target, player)
    originating_npc = target

    if target.object_id == ENPC_RYSSFLOH then
        player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
    elseif target.object_id == ENPC_GREHFARR then
        player:play_scene(target, EVENT_ID, 00004, HIDE_HOTBAR, {})
    elseif target.object_id == ENPC_BADERON then
        player:play_scene(target, EVENT_ID, 00011, CUTSCENE_FLAGS, {})
    end
end

function onYield(scene, results, player)
    -- Note that the scene yields, not returns - unlike the other openings.
    if scene == 1 then
        player:play_scene(player.id, EVENT_ID, 00002, HIDE_HOTBAR, {})
        return
    elseif scene == 4 then
        player:play_scene(originating_npc, EVENT_ID, 00005, HIDE_HOTBAR, {})
        return
    elseif scene == 6 then
        -- Move the player into the destination position
        -- TODO: fade to black with PrepareZoning to hide the teleport like in retail
        player:move_to_pop_range(POS_INN_WARP)
    elseif scene == 11 then
        player:play_scene(player.id, EVENT_ID, 00012, HIDE_HOTBAR, {})
        return
    end

    player:finish_event(EVENT_ID)
end

function onReturn(scene, results, player)
    if scene == 0 then
        -- first param: whether the quest was accepted
        local accepted = results[1] == 1
        if accepted then
            player:play_scene(originating_npc, EVENT_ID, 00001, CUTSCENE_FLAGS, {})
            return
        else
            player:play_scene(player.id, EVENT_ID, 00003, HIDE_HOTBAR, {})
            return
        end

    elseif scene == 2 then
        player:accept_quest(EVENT_ID)

        -- call back into the opening, presumably to update the borders of the play area
        player:start_event(player.id, OPENING_LIMSA, EVENT_TYPE_NEST, 0)
        player:play_scene(player.id, OPENING_LIMSA, 30, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {2})
        return
    elseif scene == 5 then
        if results[1] == 1 then
            -- take the warp
            player:play_scene(player.id, EVENT_ID, 6, HIDE_HOTBAR, {})
            return
        end
    elseif scene == 12 then
        local completed = results[1] == 1
        player:finish_quest(EVENT_ID)
    end

    player:finish_event(EVENT_ID)
end
