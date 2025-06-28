-- Event flags, courtesy of Sapphire
-- https://github.com/SapphireServer/Sapphire/blob/bf3368224a00c180cbb7ba413b52395eba58ec0b/src/world/Event/EventDefs.h#L9

-- Cutscene flags, TODO: move these to Global.lua, or maybe a new file named Cutscene.lua or something along those lines, to store all of them
FADE_OUT           = 0x00000002
HIDE_UI            = 0x00000800
HIDE_HOTBAR        = 0x00002000
CONDITION_CUTSCENE = 0x00000400
SET_BASE           = 0xF8400EFB -- Pulled from Sapphire, perhaps the default flags the server sends for most cutscenes?

-- Scene numbers
SCENE_SHOW_MENU    = 00000
SCENE_SLEEP_ANIM   = 00001
SCENE_LOG_OUT      = 00002
SCENE_DREAMFITTING = 00003
SCENE_AWAKEN_ANIM  = 00100

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, SCENE_SHOW_MENU, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    local CUTSCENE_FLAGS <const> = FADE_OUT | HIDE_UI | CONDITION_CUTSCENE
    -- Decision values the player can choose
    local NOTHING        <const> = 1
    local DREAMFITTING   <const> = 2
    local LOG_OUT        <const> = 3
    local EXIT_GAME      <const> = 4 -- LOG_OUT and EXIT_GAME are unused by us in this script, but they are provided here as documentation for the decison values
    local decision       <const> = results[1]

    if scene == SCENE_SHOW_MENU then
        if decision == NOTHING then
            player:finish_event(EVENT_ID)
        else
            player:play_scene(player.id, EVENT_ID, SCENE_SLEEP_ANIM, CUTSCENE_FLAGS, {decision})
        end
    elseif scene == SCENE_SLEEP_ANIM then
        if decision == DREAMFITTING then
            player:play_scene(player.id, EVENT_ID, SCENE_DREAMFITTING, CUTSCENE_FLAGS, {0})
        else
            -- The player decided to log out or exit the game. The server don't care which, as the client handles itself, so pass along the decision.
            player:play_scene(player.id, EVENT_ID, SCENE_LOG_OUT, CUTSCENE_FLAGS, {decision})
        end
    elseif scene == SCENE_DREAMFITTING then
        player:play_scene(player.id, EVENT_ID, SCENE_AWAKEN_ANIM, CUTSCENE_FLAGS, {0})
    else
        player:finish_event(EVENT_ID)
    end
end
