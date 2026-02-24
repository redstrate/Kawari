-- Scrip Exchange NPCs, like the one in Mor Dhona

-- Scenes
SCENE_00000 = 00000 -- Select the script to exchange (unknown args)

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
