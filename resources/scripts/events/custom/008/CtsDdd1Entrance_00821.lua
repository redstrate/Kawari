-- Wood Wailer Expeditionary Captain in South Shroud

-- Scenes
SCENE_00000 = 00000 -- Open menu
SCENE_00001 = 00001 -- Non-unlocked greeting (?)
SCENE_00002 = 00002 -- Submenu once you selected a save (?)
SCENE_00003 = 00003 -- Some story-related message (?)

-- Description UI for Palace of the Dead
DESCRIPTION_POD = 3604500

-- Content Finder Condition ID for Palace of the Dead (Floors 1-10)
CONTENT_FINDER_POD_1_10 = 174

function onTalk(target, player)
    player:play_scene(SCENE_00000, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == SCENE_00000 then
        if results[1] == 1 then
            -- TODO: support the various options

            -- Register for floors 1-10
            player:register_for_content(CONTENT_FINDER_POD_1_10)
        elseif results[1] == 5 then
            -- Open DD description menu
            player:start_event(player.id, DESCRIPTION_POD, EVENT_TYPE_NEST, 0)
            player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {})
            return
        end
    end
    player:finish_event()
end
