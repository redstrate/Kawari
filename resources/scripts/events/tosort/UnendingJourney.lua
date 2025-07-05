-- Cutscene flags, TODO: move these to Global.lua, or maybe a new file named Cutscene.lua or something along those lines, to store all of them
SET_BASE = 0xF8400EFB -- Pulled from Sapphire, perhaps the default flags the server sends for most cutscenes?

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
        player:play_scene(player.id, EVENT_ID, SCENE_PLAY_CUTSCENE, SET_BASE, results)
    elseif scene == SCENE_PLAY_CUTSCENE then
        --[[ TODO: How can we make it fade back in smoothly after the cutscene finishes?
            Could it be related to ActorControl(ViewingCutscene)? ]]
        player:play_scene(player.id, EVENT_ID, SCENE_SHOW_MENU, 8192, {1})
    else
        player:finish_event(EVENT_ID)
    end
end
