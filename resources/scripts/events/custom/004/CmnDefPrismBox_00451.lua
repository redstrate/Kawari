-- Unknown object/NPC

-- TODO: actually implement this menu

-- Scenes
SCENE_00000 = 00000 -- Cutscene 0, flags 247 ("HOW_TO_ID_2", according to Scripter) fades the screen to black for a while, then comes back after closing an invisible dialog box

function onTalk(target, player)
    -- You have not yet unlocked the glamour dresser.
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
