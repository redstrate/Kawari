-- Internally called CmnDefRetainerBell:721440

-- TODO: actually implement this menu

-- Scene 00000 - Unknown, softlocks
-- Scene 00001 - Unknown, does nothing right now
-- Scene 00002 - Softlocks, but brings up active help for "A Retainer's Many Tasks", so this is probably the bell's menu proper
-- Scene 00003 - Unknown, but it's mentioned in Scripter
-- Scene 01000 - You have not yet received approval to hire retainers.
-- Scene 01001 - You have not yet hired a retainer.
-- Scene 01002 - Unknown, does nothing right now
-- Scene 01003 - Unknown, does nothing right now

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 01000, 0, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
