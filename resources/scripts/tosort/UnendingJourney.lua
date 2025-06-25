-- TODO: actually implement this menu

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00001, 721028, 0)
    player:open_unending_journey()
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
