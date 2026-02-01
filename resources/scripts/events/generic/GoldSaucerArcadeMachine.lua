-- Generic handler for GoldSaucerArcadeMachine events

-- Scenes
SCENE_PLAY_GAME = 00014

function onTalk(target, player)
    player:play_scene(target, SCENE_PLAY_GAME, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
