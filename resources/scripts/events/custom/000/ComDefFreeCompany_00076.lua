-- Unknown object/NPC

-- Scenes
SCENE_00000 = 00000 -- How may i serve you? and then menu to found a free company
SCENE_00001 = 00001 -- Ask about FCs
SCENE_00002 = 00002 -- A company can only be disbanded by its master
SCENE_00003 = 00003 -- Recently changed allegiances
SCENE_00004 = 00004 -- Your petition was not ordered by [GC name]
SCENE_00005 = 00005 -- FC quit cooldown message
SCENE_00007 = 00007 -- Choose company tag
SCENE_00008 = 00008 -- Change name

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 then
        -- 1 means you hit the "learn about free company" button
        if results[1] == 1 then
            -- explain menu
            player:play_scene(SCENE_00001, HIDE_HOTBAR, {})
            return
        elseif results[1] == 5 then
            -- 5 means you hit the "found a free company" button

            -- reject
            player:play_scene(SCENE_00003, HIDE_HOTBAR, {})
            return
        end
    end

    player:finish_event()
end
