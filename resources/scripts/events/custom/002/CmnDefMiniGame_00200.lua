-- TODO: actually implement this menu

function onTalk(target, player)
    -- You have not yet unlocked any mini-games.
    player:play_scene(target, 00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
