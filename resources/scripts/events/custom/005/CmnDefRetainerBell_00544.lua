-- Retainer bell objects

-- TODO: actually implement this menu

-- Scenes
SCENE_00000 = 00000 -- Unknown, softlocks
SCENE_00001 = 00001 -- Unknown, does nothing right now
SCENE_00002 = 00002 -- Softlocks, but brings up active help for "A Retainer's Many Tasks", so this is probably the bell's menu proper
SCENE_00003 = 00003 -- Unknown, but it's mentioned in Scripter
SCENE_01000 = 01000 -- You have not yet received approval to hire retainers.
SCENE_01001 = 01001 -- You have not yet hired a retainer.
SCENE_01002 = 01002 -- Unknown, does nothing right now
SCENE_01003 = 01003 -- Unknown, does nothing right now

function onTalk(target, player)
    player:play_scene(target, SCENE_01000, 0, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
