-- Generic Chocobokeep NPCs

-- TODO: actually implement this menu

-- Scene
SCENE_00000 = 00000 -- Greeting, "unable to hire"?

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    player:finish_event()
end
