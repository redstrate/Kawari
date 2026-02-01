-- Generic repair NPCs

-- TODO: actually implement this menu

-- Scenes
SCENE_00000 = 00000 -- Open the repair UI

function onTalk(target, player)
    player:play_scene(target, SCENE_00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
