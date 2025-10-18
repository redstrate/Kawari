-- scene 0: how may i serve you? and then menu to found a free company
-- scene 1: ask about FCs
-- scene 2: a company can only be disbanded by its master
-- scene 3: recently changed allegiances
-- scene 4: your petition was not ordered by [GC name]
-- scene 5: FC quit cooldown message
-- scene 7: choose company tag
-- scene 8: change name

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 then
        -- 1 means you hit the "learn about free company" button
        if results[1] == 1 then
            -- explain menu
            player:play_scene(player.id, EVENT_ID, 00001, HIDE_HOTBAR, {})
            return
        elseif results[1] == 5 then
            -- 5 means you hit the "found a free company" button

            -- reject
            player:play_scene(player.id, EVENT_ID, 00003, HIDE_HOTBAR, {})
            return
        end
    end

    player:finish_event(EVENT_ID)
end
