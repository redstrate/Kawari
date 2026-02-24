-- Unknown object/NPC

-- TODO: actually implement this menu, attempting to open the "Remove an item." softlocks for now

-- Scenes
SCENE_00000 = 00000 -- Unknown

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    player:finish_event()
end
