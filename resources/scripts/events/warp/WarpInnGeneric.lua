-- Generic warp shared by all inn scripts

-- Scenes
SCENE_GREETING  = 00000 -- Initial greeting
SCENE_MENU      = 00001 -- Menu
SCENE_NO_ACCESS = 00002 -- Doesn't have inn access

function onTalk(target, player)
    player:play_scene(SCENE_GREETING, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    if scene == SCENE_GREETING then
        -- has inn access
        player:play_scene(SCENE_MENU, HIDE_HOTBAR, {0})
    else
        player:finish_event()

        if results[1] == 1 then
            -- get warp
            player:warp(EVENT_ID)
        end
    end
end
