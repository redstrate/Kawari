-- TODO: open the shop menus when requested. this is a specialshop, but requires event nesting

-- scene 0: menu
-- scene 1: obtain the mogpendium
-- scene 2: open the mogpendium only

function onTalk(target, player, game_data)
    player:play_scene(target, EVENT_ID, 0, 0, {0})
end

function onReturn(scene, results, player)
    if scene == 0 then
        -- request to open a tomestone shop menu
        if results[1] == 1 and #results > 1 then
            print("Getting target shop!")
            local target_shop_id = results[2]

            player:start_event(player.id, target_shop_id, EVENT_TYPE_NEST, 5)
            player:play_scene(player.id, target_shop_id, 0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
            return
        end
    end
    player:finish_event(EVENT_ID)
end
