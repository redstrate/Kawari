-- TODO: open the shop menus when requested. this is a specialshop, but requires event nesting

-- scene 0: menu
-- scene 1: obtain the mogpendium
-- scene 2: open the mogpendium only

function onTalk(target, player, game_data)
    player:play_scene(target, EVENT_ID, 0, 0, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID, 0)
end
