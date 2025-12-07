-- Seasonal Quartermaster in Wolves Den Pier

-- Scene 0: Initial greeting
-- Scene 1: Open menu
-- Scene 2: Open Item Request menu
-- Scene 3: No reward message
-- Scene 5: Nested help menu
-- Scene 10: Previous season rewards have already been claimed
-- Scene 11: Inventory is full
-- Scene 12: Some congratulations message
-- Scene 13: No valid voucher
-- Scene 14: No rewards are available for preseason rankings
-- Scene 15: Unable to obtain trophy crystals

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 then
        -- Open menu
        player:play_scene(player.id, EVENT_ID, 1, HIDE_HOTBAR, {})
        return
    elseif scene == 1 then
        if results[1] == 1 then
            -- No reward
            player:play_scene(player.id, EVENT_ID, 3, HIDE_HOTBAR, {})
            return
        elseif results[1] == 2 then
            -- No reward
            player:play_scene(player.id, EVENT_ID, 3, HIDE_HOTBAR, {})
            return
        elseif results[1] == 3 then
            -- Open nested help menu
            player:play_scene(player.id, EVENT_ID, 5, HIDE_HOTBAR, {})
            return
        end
    end
    player:finish_event(EVENT_ID)
end
