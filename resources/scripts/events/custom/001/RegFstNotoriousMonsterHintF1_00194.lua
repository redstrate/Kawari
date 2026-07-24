-- Hunter-scholar in Central Shroud

-- Scenes
SCENE_GREETING = 00000
SCENE_MENU = 00001

function onTalk(target, player)
    player:play_scene(SCENE_GREETING, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    if scene == SCENE_GREETING then
        player:play_scene(SCENE_MENU, HIDE_HOTBAR, {0})
        return
    end
    player:finish_event()
end
