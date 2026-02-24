-- Baldin at Moraby Drydocks

-- Scenes
SCENE_00000 = 00000 -- Initial greeting
SCENE_00010 = 00010 -- Menu
SCENE_00020 = 00020 -- Acquaintance selection
SCENE_00030 = 00030 -- Help menu

function onTalk(target, player)
    player:play_scene(SCENE_00000, 0, {0})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 then
        if results[1] == 2 then
            player:change_territory(1055)
        elseif results[1] == 3 then
            -- Open selection menu
            player:play_scene(SCENE_00020, 0, {0})
            return
        elseif results[1] == 5 then
            -- Open help menu
            player:play_scene(SCENE_00030, 0, {0})
            return
        end
    end

    player:finish_event()
end
