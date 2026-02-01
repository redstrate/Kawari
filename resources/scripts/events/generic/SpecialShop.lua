-- TODO: actually implement hunt currency and possible opcodes for doing the transactions

function onTalk(target, player)
    player:play_scene(target, 00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
