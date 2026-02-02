-- Masked Rose in Gold Saucer

-- Scenes
SCENE_00000 = 00000 -- Generic greeting (not unlocked?)
SCENE_00001 = 00001 -- Unknown (doesn't play)
SCENE_00002 = 00002 -- Judging beginning cutscene
SCENE_00003 = 00003 -- Prize Reward
SCENE_00004 = 00004 -- Bonus reward
SCENE_00005 = 00005 -- End message
SCENE_00006 = 00006 -- Unknown
SCENE_00007 = 00007 -- Prize + Bonus reward
SCENE_00010 = 00010 -- Some weird message, internally called "linkdeadreturn"?!

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    player:finish_event()
end
