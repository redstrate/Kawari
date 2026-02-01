-- Wood Wailer Expeditionary Captain in South Shroud

-- Scene 0: Open menu
-- Scene 1: Non-unlocked greeting (?)
-- Scene 2: Submenu once you selected a save (?)
-- Scene 3: Some story-related message (?)

-- Description UI for Palace of the Dead
DESCRIPTION_POD = 3604500

-- Content Finder Condition ID for Palace of the Dead (Floors 1-10)
CONTENT_FINDER_POD_1_10 = 174

function onTalk(target, player)
    player:play_scene(target, 0, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 then
        if results[1] == 1 then
            -- TODO: support the various options

            -- Register for floors 1-10
            player:register_for_content(CONTENT_FINDER_POD_1_10)
        elseif results[1] == 5 then
            -- Open DD description menu
            player:start_event(player.id, DESCRIPTION_POD, EVENT_TYPE_NEST, 0)
            player:play_scene(player.id, 0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
            return
        end
    end
    player:finish_event()
end
