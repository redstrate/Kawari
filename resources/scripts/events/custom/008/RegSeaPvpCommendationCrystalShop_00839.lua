-- Commendation Quartermaster
-- TODO: this nests something i don't know yet'

function onTalk(target, player)
    player:play_scene(target, 0, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
