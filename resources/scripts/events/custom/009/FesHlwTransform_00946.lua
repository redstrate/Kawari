-- Scene 0: Show menu, depends on quest completion

EFFECT_TRANSFIGURATION = 565
EFFECT_DURATION = 1800.0 -- as seen in retail

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, 0, {0})
end

function onReturn(scene, results, player)
    if scene == 0 and results[1] > 0 then
       -- first param is your transformation selection
       local effect_param_id = GAME_DATA:get_halloween_npc_transform(results[1])
       player:gain_effect(EFFECT_TRANSFIGURATION, effect_param_id, EFFECT_DURATION)
    end
    player:finish_event(EVENT_ID)
end
