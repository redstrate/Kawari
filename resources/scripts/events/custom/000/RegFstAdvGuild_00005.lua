-- Unknown object/NPC

-- Scenes
SCENE_00000 = 00000 -- Default greeting
SCENE_00001 = 00001 -- Regular menu asking stuff like "What do you do here?"

function onTalk(target, player)
    player:play_scene(target, SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == SCENE_00000 then
        player:play_scene(player.id, SCENE_00001, HIDE_HOTBAR, {})
    else
        player:finish_event()
    end
end
