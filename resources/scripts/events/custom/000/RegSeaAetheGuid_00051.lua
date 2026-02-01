-- Sundhimal in Limsa Lominsa

-- Scenes
SCENE_00000 = 00000 -- Initial greeting
SCENE_00001 = 00001 -- Menu asking about aetherytes

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
