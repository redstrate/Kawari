-- TODO: actually implement hunt currency and possible opcodes for doing the transactions

function onTalk(target, player, game_data)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
