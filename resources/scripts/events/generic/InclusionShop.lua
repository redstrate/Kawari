-- TODO: actually implement currency and possible opcodes for doing the transactions

function onTalk(target, player)
    player:play_scene(target, 1, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
