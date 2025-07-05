-- Cutscene flags, TODO: move these to Global.lua, or maybe a new file named Cutscene.lua or something along those lines, to store all of them
SET_BASE = 0xF8400EFB -- Pulled from Sapphire, perhaps the default flags the server sends for most cutscenes?

SCENE_SHOW_MENU       = 00000
SCENE_PLAY_CUTSCENE   = 00001

function onTalk(target, player)
    -- you cannot consult, which is good because we don't know how to implement this anyway
    player:play_scene(target, EVENT_ID, SCENE_SHOW_MENU, 8192, {0})
end

function onReturn(scene, results, player)
    -- A result of zero means the user exited the menu without playing anything.
    if scene == 0 and results[1] ~= 0 then
        -- TODO: we need to switch the player into viewingcutscene online status (on the Rust side?)
        player:play_scene(player.id, EVENT_ID, SCENE_PLAY_CUTSCENE, SET_BASE, results)
        return
        -- TODO: we cannot nest cutscenes right now, but control should return back to the UEJ menu when a cutscene finishes
    end
    player:finish_event(EVENT_ID)
end
