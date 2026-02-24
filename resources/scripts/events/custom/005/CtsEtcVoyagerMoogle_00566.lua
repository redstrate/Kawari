-- Unknown object/NPC

-- TODO: open the shop menus when requested. this is a specialshop, but requires event nesting

-- Scenes
SCENE_00000 = 00000 -- Menu
SCENE_00001 = 00001 -- Obtain the mogpendium
SCENE_00002 = 00002 -- Open the mogpendium only

function onTalk(target, player)
    player:play_scene(SCENE_00000, 0, {0})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 then
        -- request to open a tomestone shop menu
        if results[1] == 1 and #results > 1 then
            local target_shop_id = results[2]

            player:start_event(target_shop_id, EVENT_TYPE_NEST, 5)
            player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
            return
        end
    end
    player:finish_event()
end
