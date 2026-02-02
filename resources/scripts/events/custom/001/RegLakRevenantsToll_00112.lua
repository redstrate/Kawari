-- Alison in Mor Dhona

-- Scenes
SCENE_00000 = 00000 -- Initial greeting
SCENE_00001 = 00001 -- Help menu

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == SCENE_00000 then
        -- Show help menu
        player:play_scene(SCENE_00001, HIDE_HOTBAR, {})
        return
    end
    player:finish_event()
end
