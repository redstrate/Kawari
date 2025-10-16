-- Scene 0: Default greeting
-- Scene 1: Regular menu asking stuff like "What do you do here?"

function onTalk(target, player, game_data)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == 0 then
        player:play_scene(player.id, EVENT_ID, 00001, HIDE_HOTBAR, {})
    else
        player:finish_event(EVENT_ID, 0)
    end
end
