-- J'nasshym in Limsa Lominsa

-- scene 0: greeting

function onTalk(target, player)
    player:play_scene(target, 00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
