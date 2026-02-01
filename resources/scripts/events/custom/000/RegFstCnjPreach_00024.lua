-- scene 0: greeting
-- scene 1: menu asking about elementals

function onTalk(target, player)
    player:play_scene(target, 00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 then
        player:play_scene(player.id, 00001, HIDE_HOTBAR, {})
    else
        player:finish_event()
    end
end
