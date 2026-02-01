-- Commendation Quartermaster

-- TODO: this nests something i don't know yet'

-- Scenes
SCENE_00000 = 00000

function onTalk(target, player)
    player:play_scene(target, SCENE_00000, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
