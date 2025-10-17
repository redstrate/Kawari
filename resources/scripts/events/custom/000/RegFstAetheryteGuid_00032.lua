-- scene 0: greeting
-- scene 1: menu asking about aetherytes

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == 0 then
        player:play_scene(player.id, EVENT_ID, 00001, HIDE_HOTBAR, {})
    else
        player:finish_event(EVENT_ID)
    end
end
