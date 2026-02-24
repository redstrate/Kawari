-- Generic handler for GimmickBill events

function onTalk(target, player)
    player:play_scene(1, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    player:finish_event()
end
