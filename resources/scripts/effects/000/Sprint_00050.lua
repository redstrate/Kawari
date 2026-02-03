EFFECT_JOG = 4209

function onGain(player)
    -- it does nothing
end

function onLose(player)
    player:gain_effect(EFFECT_JOG, 20, 0.0)
end
