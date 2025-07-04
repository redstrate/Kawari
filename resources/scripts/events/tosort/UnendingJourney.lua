SCENE_SHOW_MENU       = 00000
SCENE_PLAY_CUTSCENE   = 00001

function onTalk(target, player)
    -- you cannot consult, which is good because we don't know how to implement this anyway
    player:play_scene(target, EVENT_ID, SCENE_SHOW_MENU, 8192, {0})
end

function onReturn(scene, results, player)
    if scene == 0 then
        -- TODO: this is not the correct cutscene flags
        -- TODO: we need to switch the player into viewingcutscene online status
        player:play_scene(player.id, EVENT_ID, SCENE_PLAY_CUTSCENE, 8192, results)
        return
    end
    player:finish_event(EVENT_ID)
end
