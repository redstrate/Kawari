-- scene 0: introduction to linkshells/greeting
-- scene 1: linkshell management menu
-- scene 2: create personal linkshell
-- scene 3: rename deny
-- scene 4: disband deny

function onTalk(target, player, game_data)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == 0 then
        player:play_scene(player.id, EVENT_ID, 00001, HIDE_HOTBAR, {})
        return
    elseif scene == 1 then
        -- create linkshell
        if results[1] == 2 then
            player:play_scene(player.id, EVENT_ID, 00002, HIDE_HOTBAR, {})

            -- NOTE: we intentionally end here because we don't handle the linkshell creation packet and it will freeze the event
        elseif results[1] == 3 then
            -- rename linkshell

            -- deny for now
            player:play_scene(player.id, EVENT_ID, 00003, HIDE_HOTBAR, {})
            return
        elseif results[1] == 4 then
            -- disband linkshell

            -- deny for now
            player:play_scene(player.id, EVENT_ID, 00004, HIDE_HOTBAR, {})
            return
        end
    end

    player:finish_event(EVENT_ID, 0)
end
