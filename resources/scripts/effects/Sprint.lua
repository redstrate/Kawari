EFFECT_JOG = 4029

function onGain(player)
    -- it does nothing
end

function onLose(player)
    player:gain_effect(EFFECT_JOG)
end
