-- Internally called CmnDefBeautySalon:721044

-- TODO: actually implement this menu

-- Scene 00000: "You are not authorized to summon the aesthetician.", also seems to be the prompt cutscene, but still unsure how to get the prompt to appear
-- Scene 00001: Aesthetician appears and speaks, then scene 2 would begin to play, but it probably needs server-side help?
-- Scene 00002: Softlocks and does nothing, seems to be where you'd be taken to the makeover menus to actually change your appearance
-- Scene 00003: End of using bell where aesthetician has rushed past the player with his scissors animation, then walks off

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    if scene == 0 then
        -- results[1] is 1 if you want to summon, otherwise 0
        if results[1] == 1 then
            player:play_scene(player.id, EVENT_ID, 00001, FADE_OUT | HIDE_UI | CONDITION_CUTSCENE, {0})
            return
        end
    elseif scene == 1 then
        player:play_scene(player.id, EVENT_ID, 00002, FADE_OUT | HIDE_UI | CONDITION_CUTSCENE, {0})
        return
    elseif scene == 2 then
        player:play_scene(player.id, EVENT_ID, 00003, FADE_OUT | HIDE_UI | CONDITION_CUTSCENE, {0})
        return
    end
    player:finish_event(EVENT_ID)
end
