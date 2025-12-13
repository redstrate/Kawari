-- Collectable Appraiser in Mor Dhona

-- Scene 0: Show menu

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 and results[1] == 1 then
        -- They give us the ID of the nested event (a CollectablesShop event, of course!)
        local target_event_id = results[2]
        player:start_event(player.id, target_event_id, EVENT_TYPE_NEST, 0)
        player:play_scene(player.id, target_event_id, 0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
        return
    end
    player:finish_event(EVENT_ID)
end
