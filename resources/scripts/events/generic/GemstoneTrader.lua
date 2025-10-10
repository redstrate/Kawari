-- TODO: actually implement bicolor gemstones, and present a 'shop' menu (it seems to be different than hunt exchange NPCs)?

--Scene 00000 "Collect bicolor gemstones to trade for a variety of goods" / scene informing the player they can now access the shop
--Scene 00001 softlocks or simply shows a dialog box (depends per vendor), presumably needs shop params sent to it
--Scene 00002 does nothing (for now)
--Scene 00003 "You must progress further through the <expansion name> main scenario in order to access this vendor's wares."

function onTalk(target, player)
    -- "You must progress further through the <expansion name> main scenario in order to access this vendor's wares."
    player:play_scene(target, EVENT_ID, 00003, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID, 0)
end
