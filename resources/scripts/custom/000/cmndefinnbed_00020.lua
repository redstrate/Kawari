--- TODO: find a way to hardcode it this way
EVENT_ID = 720916

-- TODO: in retail, there is a fade in/out between the prompt and the sleep anim?

function onTalk(target, player)
    --- prompt the bed menu
    player:play_scene(target, EVENT_ID, 0, 8192, 0)
end

function onReturn(scene, results, player)
    if scene == 0 then -- prompt
        if results[1] == 1 then
            -- nothing
        elseif results[1] == 2 then
            -- dreamfitting not implemented
        elseif results[1] == 3 then
            -- play sleep animation
            player:play_scene(player.id, EVENT_ID, 1, 8192, 0)
            player:begin_log_out()
            return
        elseif results[1] == 4 then
            -- play sleep animation
            player:play_scene(player.id, EVENT_ID, 1, 8192, 0)
            player:begin_log_out()
            return
        end

        player:finish_event(EVENT_ID)
    elseif scene == 1 then -- sleep anim
        -- play log out scene
        player:play_scene(player.id, EVENT_ID, 2, 8192, 0)
    elseif scene == 2 then -- log out
        player:finish_event(EVENT_ID)
    end
end
