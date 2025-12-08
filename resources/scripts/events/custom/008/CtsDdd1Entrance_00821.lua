-- Wood Wailer Expeditionary Captain in South Shroud

-- Scene 0: Open menu
-- Scene 1: Non-unlocked greeting (?)
-- Scene 2: Submenu once you selected a save (?)
-- Scene 3: Some story-related message (?)

DESCRIPTION_POD = 3604500

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 then
        if results[1] == 5 then
            -- Open DD description menu
            player:start_event(player.id, DESCRIPTION_POD, EVENT_TYPE_NEST, 0)
            player:play_scene(player.id, DESCRIPTION_POD, 0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
            return
        end
    end
    player:finish_event(EVENT_ID)
end
