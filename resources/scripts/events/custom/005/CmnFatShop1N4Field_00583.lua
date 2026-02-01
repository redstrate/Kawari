-- Generic FATE vendor shop NPCs in open world areas

-- TODO: actually implement bicolor gemstones, and present a 'shop' menu (it seems to be different than hunt exchange NPCs)?
-- TODO: maybe de-duplicate these files

-- Scenes
SCENE_00000 = 00000 -- "Collect bicolor gemstones to trade for a variety of goods" / scene informing the player they can now access the shop
SCENE_00001 = 00001 -- Softlocks or simply shows a dialog box (depends per vendor), presumably needs shop params sent to it
SCENE_00002 = 00002 -- Does nothing (for now)
SCENE_00003 = 00003 -- "You must progress further through the <expansion name> main scenario in order to access this vendor's wares."

function onTalk(target, player)
    -- "You must progress further through the <expansion name> main scenario in order to access this vendor's wares."
    player:play_scene(target, SCENE_00003, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
