-- Masked Rose in Gold Saucer

-- Scene 0: Generic greeting (not unlocked?)
-- Scene 1: Unknown (doesn't play)
-- Scene 2: Judging beginning cutscene
-- Scene 3: Prize Reward
-- Scene 4: Bonus reward
-- Scene 5: End message
-- Scene 6: Unknown
-- Scene 7: Prize + Bonus reward
-- Scene 10: Some weird message, internally called "linkdeadreturn"?!

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event(EVENT_ID)
end
