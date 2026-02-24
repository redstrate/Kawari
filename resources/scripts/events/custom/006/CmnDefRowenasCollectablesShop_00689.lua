-- Collectable Appraiser in Mor Dhona

-- Scenes
SCENE_00000 = 00000 -- Show menu

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 and results[1] == 1 then
        -- They give us the ID of the nested event (a CollectablesShop event, of course!)
        local target_event_id = results[2]
        player:start_event(target_event_id, EVENT_TYPE_NEST, 0)
        player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
        return
    end
    player:finish_event()
end
