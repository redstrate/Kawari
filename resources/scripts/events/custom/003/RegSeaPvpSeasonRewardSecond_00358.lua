-- Seasonal Quartermaster in Wolves Den Pier

-- Scenes
SCENE_00000 = 00000 -- Initial greeting
SCENE_00001 = 00001 -- Open menu
SCENE_00002 = 00002 -- Open Item Request menu
SCENE_00003 = 00003 -- No reward message
SCENE_00005 = 00005 -- Nested help menu
SCENE_00010 = 00010 -- Previous season rewards have already been claimed
SCENE_00011 = 00011 -- Inventory is full
SCENE_00012 = 00012 -- Some congratulations message
SCENE_00013 = 00013 -- No valid voucher
SCENE_00014 = 00014 -- No rewards are available for preseason rankings
SCENE_00015 = 00015 -- Unable to obtain trophy crystals

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 then
        -- Open menu
        player:play_scene(1, HIDE_HOTBAR, {})
        return
    elseif scene == SCENE_00001 then
        if results[1] == 1 then
            -- No reward
            player:play_scene(3, HIDE_HOTBAR, {})
            return
        elseif results[1] == 2 then
            -- No reward
            player:play_scene(3, HIDE_HOTBAR, {})
            return
        elseif results[1] == 3 then
            -- Open nested help menu
            player:play_scene(5, HIDE_HOTBAR, {})
            return
        end
    end
    player:finish_event()
end
