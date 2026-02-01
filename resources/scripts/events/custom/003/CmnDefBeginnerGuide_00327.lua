-- Unknown object/NPC

-- Scenes
SCENE_00000 = 00000 -- Show the menu, the rest is handled in client-side Lua

function onTalk(target, player)
    player:play_scene(target, 00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
