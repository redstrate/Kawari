-- Crystal bell object

-- Scenes
SCENE_00000 = 00000 -- "You are not authorized to summon the aesthetician.", also seems to be the prompt cutscene, but still unsure how to get the prompt to appear
SCENE_00001 = 00001 -- Aesthetician appears and speaks, then scene 2 would begin to play, but it probably needs server-side help?
SCENE_00002 = 00002 -- Softlocks and does nothing, seems to be where you'd be taken to the makeover menus to actually change your appearance
SCENE_00003 = 00003 -- End of using bell where aesthetician has rushed past the player with his scissors animation, then walks off

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 then
        -- results[1] is 1 if you want to summon, otherwise 0
        if results[1] == 1 then
            player:play_scene(SCENE_00001, FADE_OUT | HIDE_UI | CONDITION_CUTSCENE, {})
            return
        end
    elseif scene == SCENE_00001 then
        player:play_scene(SCENE_00002, HIDE_HOTBAR, {})
        return
    elseif scene == SCENE_00002 then
        -- Used to determine whether to play the full animation or not, if cancelled it doesn't'
        local result = 0
        if results[1] == 1 then
            result = 1
        end
        player:play_scene(SCENE_00003, FADE_OUT | HIDE_UI | CONDITION_CUTSCENE, {result})
        return
    end
    player:finish_event()
end

function onYield(scene, id, results, player)
    if scene == SCENE_00002 and id == 27 then
        local customize = player.chara_make.customize
        customize.race = race_from_repr(results[2])
        customize.gender = gender_from_repr(results[3])
        customize.age = results[4]
        customize.height = results[5]
        customize.tribe = tribe_from_repr(results[6])
        customize.face = results[7]
        customize.hair = results[8]
        customize.enable_highlights = results[9] == 1
        customize.skin_tone = results[10]
        customize.right_eye_color = results[11]
        customize.hair_tone = results[12]
        customize.highlights = results[13]
        customize.facial_features = results[14]
        customize.facial_feature_color = results[15]
        customize.eyebrows = results[16]
        customize.left_eye_color = results[17]
        customize.eyes = results[18]
        customize.nose = results[19]
        customize.jaw = results[20]
        customize.mouth = results[21]
        customize.lips_tone_fur_pattern = results[22]
        customize.race_feature_size = results[23]
        customize.race_feature_type = results[24]
        customize.bust = results[25]
        customize.face_paint = results[26]
        customize.face_paint_color = results[27]

        player:set_customize(customize)

        -- We don't need to pass all of this back to the client
        player:resume_event(scene, id, {1})
        return
    end
    player:resume_event(scene, id, results)
end
