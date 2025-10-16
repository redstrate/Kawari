-- TODO: actually implement this menu

function onTalk(target, player, game_data)
    -- You have not yet unlocked any mini-games.
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
