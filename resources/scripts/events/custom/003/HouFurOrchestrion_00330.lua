-- Orchestrion object

-- Scenes
SCENE_00000 = 00000 -- Opens the main player
SCENE_00001 = 00001 -- Opens the playlist editor, but right now, closing it softlocks, and trying to edit anything says you are not authorized to use the estate's orchestrion

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    player:finish_event()
end
