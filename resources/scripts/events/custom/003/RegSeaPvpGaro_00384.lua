-- Disreputable Priest in Wolves Den Pier

-- Scenes
SCENE_00000 = 00000 -- Open shop menu
SCENE_00001 = 00001 -- Acquired all titles
SCENE_00002 = 00002 -- Some other title acquisition response
SCENE_00003 = 00003 -- Acquiring a new title
SCENE_00100 = 00100 -- "That's not what I'm looking for" message

function onTalk(target, player)
    player:play_scene(target, SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    -- TODO: implement the shop

    player:finish_event()
end
