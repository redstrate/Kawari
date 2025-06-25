-- Internally called CmnDefBeautySalon:721044

-- Event flags, courtesy of Sapphire
-- https://github.com/SapphireServer/Sapphire/blob/bf3368224a00c180cbb7ba413b52395eba58ec0b/src/world/Event/EventDefs.h#L9
FADE_OUT = 0x00000002
HIDE_UI = 0x00000800
HIDE_HOTBAR = 0x2000 -- 8192
CONDITION_CUTSCENE = 0x00000400
SET_BASE = 0xF8400EFB

-- TODO: actually implement this menu

-- Scene 00000: "You are not authorized to summon the aesthetician.", also seems to be the prompt cutscene, but still unsure how to get the prompt to appear
-- Scene 00001: Aesthetician appears and speaks, then scene 2 would begin to play, but it probably needs server-side help?
-- Scene 00002: Softlocks and does nothing, seems to be where you'd be taken to the makeover menus to actually change your appearance
-- Scene 00003: End of using bell where aesthetician has rushed past the player with his scissors animation, then walks off

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, 0)
end

function onReturn(scene, results, player)
    if scene == 1 then
        player:play_scene(player.id, EVENT_ID, 00002, FADE_OUT + HIDE_UI + CONDITION_CUTSCENE, 0)
    elseif scene == 2 then
        player:play_scene(player.id, EVENT_ID, 00003, FADE_OUT + HIDE_UI + CONDITION_CUTSCENE, 0)
    end
    player:finish_event(EVENT_ID)
end
