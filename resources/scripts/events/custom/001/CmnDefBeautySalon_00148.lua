-- Crystal bell object

-- Scenes
SCENE_00000 = 00000 -- "You are not authorized to summon the aesthetician.", also seems to be the prompt cutscene, but still unsure how to get the prompt to appear
SCENE_00001 = 00001 -- Aesthetician appears and speaks, then scene 2 would begin to play, but it probably needs server-side help?
SCENE_00002 = 00002 -- Aesthetician editor/payment scene
SCENE_00003 = 00003 -- End of using bell where aesthetician has rushed past the player with his scissors animation, then walks off

-- Retail scene flags captured from CmnDefBeautySalon_00148.
SCENE_00001_FLAGS = 0xF8482EF3
SCENE_00003_FLAGS = 0xF8482EF1

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 then
        -- results[1] is 1 if you want to summon, otherwise 0
        if results[1] == 1 then
            player:play_scene(SCENE_00001, SCENE_00001_FLAGS, {})
            return
        end
    elseif scene == SCENE_00001 then
        player:play_scene(SCENE_00002, HIDE_HOTBAR, {})
        return
    elseif scene == SCENE_00002 then
        player:play_scene(SCENE_00003, SCENE_00003_FLAGS, {results[1] or 0})
        return
    end
    player:finish_event()
end

function onYield(scene, id, results, player)
    player:resume_event(scene, id, results)
end
