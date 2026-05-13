-- Osmon in Old Sharlayan

-- Scenes
SCENE_00000 = 00000 -- unknown
SCENE_00001 = 00001 -- unknown
SCENE_00010 = 00010 -- "I need a new novel to read that's truly novel" is probably when he has nothing to say?

function onTalk(target, player)
    -- The only thing that reliably works, most likely due to quest requirements?
    player:play_scene(SCENE_00010, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end
