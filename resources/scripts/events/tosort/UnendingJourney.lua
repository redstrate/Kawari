-- Cutscene flags, TODO: move these to Global.lua, or maybe a new file named Cutscene.lua or something along those lines, to store all of them
--[[ Captured from retail, this allows smooth fading in and out on both sides of the cutscene, and it would be a good idea
to see what flags are set that make it different than SET_BASE, but for now this makes the Unending Journey as accurate
as it possibly can be on our end. ]]
UEJ_REPLAY_FLAGS = 0xf8c82efb

SCENE_SHOW_MENU       = 00000
SCENE_PLAY_CUTSCENE   = 00001

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, SCENE_SHOW_MENU, 8192, {0})
end

function onReturn(scene, results, player)
    local NO_SCENE <const> = 0
    local decision <const> = results[1]

    if scene == SCENE_SHOW_MENU and decision ~= NO_SCENE then
        -- TODO: we need to switch the player into viewingcutscene online status (on the Rust side?)
        player:play_scene(player.id, EVENT_ID, SCENE_PLAY_CUTSCENE, UEJ_REPLAY_FLAGS, results)
    elseif scene == SCENE_PLAY_CUTSCENE then
        player:play_scene(player.id, EVENT_ID, SCENE_SHOW_MENU, 8192, {1})
    else
        player:finish_event(EVENT_ID)
    end
end
