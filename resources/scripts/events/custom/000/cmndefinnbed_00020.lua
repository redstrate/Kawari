--- TODO: find a way to hardcode it this way
EVENT_ID = 720916

-- Event flags, courtesy of Sapphire
-- https://github.com/SapphireServer/Sapphire/blob/bf3368224a00c180cbb7ba413b52395eba58ec0b/src/world/Event/EventDefs.h#L9
FADE_OUT = 0x00000002
HIDE_UI = 0x00000800
HIDE_HOTBAR = 0x2000 -- 8192
CONDITION_CUTSCENE = 0x00000400
SET_BASE = 0xF8400EFB

SHOW_MENU = 00000
SLEEP_ANIM = 00001
LOG_OUT = 00002
DREAMFITTING = 00003
EXIT_GAME = 00004
WAKE_UP_ANIM = 00100

-- TODO: in retail, there is a fade in/out between the prompt and the sleep anim?


function onTalk(target, player)
    player:play_scene(target, EVENT_ID, SHOW_MENU, HIDE_HOTBAR, 0)
end

function onReturn(scene, results, player)
    if scene == SHOW_MENU then -- prompt
        if results[1] == 1 then
            -- nothing
        elseif results[1] == 2 then
            -- Dreamfitting partially implemented. It works completely when played in onTalk, but does not trigger in onReturn. Unsure why.
            player:play_scene(player.id, EVENT_ID, DREAMFITTING, FADE_OUT + HIDE_UI + CONDITION_CUTSCENE, 0)        
        elseif results[1] == 3 then -- log out
            player:play_scene(player.id, EVENT_ID, SLEEP_ANIM, FADE_OUT + HIDE_UI + CONDITION_CUTSCENE, 0)
            player:begin_log_out()
            return
        elseif results[1] == 4 then -- exit game
            player:play_scene(player.id, EVENT_ID, SLEEP_ANIM, FADE_OUT + HIDE_UI + CONDITION_CUTSCENE, 0)
            player:begin_log_out()
            return
        end

        player:finish_event(EVENT_ID)
    elseif scene == SLEEP_ANIM then
        -- play log out scene
        player:play_scene(player.id, EVENT_ID, LOG_OUT, FADE_OUT + HIDE_UI + CONDITION_CUTSCENE, 0)
    elseif scene == LOG_OUT then
        player:finish_event(EVENT_ID)
    elseif scene == DREAMFITTING then
       player:play_scene(player.id, EVENT_ID, WAKE_UP_ANIM, FADE_OUT + HIDE_UI + CONDITION_CUTSCENE, 0)
    elseif scene == WAKE_UP_ANIM then -- wake up anim
        player:finish_event(EVENT_ID)
    end
end
