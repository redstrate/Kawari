-- Linkshell distributor NPCs

-- Scenes
SCENE_00000 = 00000 -- Introduction to linkshells/greeting
SCENE_00001 = 00001 -- Linkshell management menu
SCENE_00002 = 00002 -- Create personal linkshell
SCENE_00003 = 00003 -- Rename deny
SCENE_00004 = 00004 -- Disband deny

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 then
        player:play_scene(SCENE_00001, HIDE_HOTBAR, {})
        return
    elseif scene == SCENE_00001 then
        -- create linkshell
        if results[1] == 2 then
            player:play_scene(SCENE_00002, HIDE_HOTBAR, {})

            -- NOTE: we intentionally end here because we don't handle the linkshell creation packet and it will freeze the event
        elseif results[1] == 3 then
            -- rename linkshell

            -- deny for now
            player:play_scene(SCENE_00003, HIDE_HOTBAR, {})
            return
        elseif results[1] == 4 then
            -- disband linkshell

            -- deny for now
            player:play_scene(SCENE_00004, HIDE_HOTBAR, {})
            return
        end
    end

    player:finish_event()
end
