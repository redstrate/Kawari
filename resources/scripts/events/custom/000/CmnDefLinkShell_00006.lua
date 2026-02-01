-- scene 0: introduction to linkshells/greeting
-- scene 1: linkshell management menu
-- scene 2: create personal linkshell
-- scene 3: rename deny
-- scene 4: disband deny

function onTalk(target, player)
    player:play_scene(target, 00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 then
        player:play_scene(player.id, 00001, HIDE_HOTBAR, {})
        return
    elseif scene == 1 then
        -- create linkshell
        if results[1] == 2 then
            player:play_scene(player.id, 00002, HIDE_HOTBAR, {})

            -- NOTE: we intentionally end here because we don't handle the linkshell creation packet and it will freeze the event
        elseif results[1] == 3 then
            -- rename linkshell

            -- deny for now
            player:play_scene(player.id, 00003, HIDE_HOTBAR, {})
            return
        elseif results[1] == 4 then
            -- disband linkshell

            -- deny for now
            player:play_scene(player.id, 00004, HIDE_HOTBAR, {})
            return
        end
    end

    player:finish_event()
end
