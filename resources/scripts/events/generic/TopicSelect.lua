-- Generic handler for TopicSelect events

-- These are basically used for submenus.
-- For example, the Battle Supplier in Limsa has a couple of events for DoW and DoM gear.
-- Then, these events lead into a TopicSelect used for the various level categories.

function onTalk(target, player)
    player:play_scene(00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    -- first result is the selected topic
    local selected_topic = results[1]
    if selected_topic == -1 then
        player:finish_event()
        return
    end

    local target_event_id = GAME_DATA:get_topic_select_target(EVENT_ID, selected_topic)

    player:start_event(target_event_id, EVENT_TYPE_NEST, 0)
    -- this is just a limitation in the scripting API
    -- because prehandler only listens to onTalk, but that's obviously never called during nesting
    player:start_talk_event()
end
