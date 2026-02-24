-- Generic handler for GoldSaucerArcadeMachine events

-- Scenes
SCENE_PLAY_GAME = 00014

function onTalk(target, player)
    player:play_scene(SCENE_PLAY_GAME, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end

function onYield(scene, results, player)
    if results[1] == 0 then
        local timeout = 15 -- TODO: fetch from lua
        player:resume_event(SCENE_PLAY_GAME, {1, os.time() + timeout, 2000, 0})
    end
end
