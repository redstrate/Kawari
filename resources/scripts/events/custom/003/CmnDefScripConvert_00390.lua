-- Scrip Exchange NPCs, like the one in Mor Dhona

-- Scene 0: Select the script to exchange (unknown args)

function onTalk(target, player)
    player:play_scene(target, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
