-- These are basically used for submenus.
-- For example, the Battle Supplier in Limsa has a couple of events for DoW and DoM gear.
-- Then, these events lead into a TopicSelect used for the various level categories.

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, 00000, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    -- first result is the selected topic
    local selected_topic = results[1]
    player:finish_event(EVENT_ID)

    -- TODO: start the new shop event
    player:send_message("Submenu shops aren't implemented yet!")
end
