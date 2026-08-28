-- Blunderville Registrar in Blunderville

-- Scenes
SCENE_00000 = 00000 -- Open menu

-- Content Finder Condition ID for Blunderville
CONTENT_FINDER_CONDITION = 958

function onTalk(target, player)
    player:play_scene(SCENE_00000, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 and #results == 2 then
        player:register_for_content(CONTENT_FINDER_CONDITION)
    end
    player:finish_event()
end
