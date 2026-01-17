-- Basically, all Event NPCs reference a specific sheet. You can see this in ENpcData in the ENpcBase sheet.
-- Events are then run through the dispatcher, which references either a generic script or a custom one.

-- Extracts the script id from a given CustomTalk name. For example, "CmnDefBeginnerGuide_00327" will return 327.
function extractScriptId(name)
    return tonumber(name:sub(-5))
end

-- Creates the proper folder name from a given script id. For example, 327 will return 003.
function folderFromScriptId(id)
    return string.format("%03d", math.floor(id / 100))
end

-- This is called whenever the client requests to start an event
function dispatchEvent(player, event_id)
    local event_type = event_id >> 16

    -- 'Normal' events
    if event_type == HANDLER_TYPE_QUEST then
        local script_name = GAME_DATA:get_quest_name(event_id)
        local script_id = extractScriptId(script_name)
        local script_folder = folderFromScriptId(script_id)
        local script_path = "events/quest/"..script_folder.."/"..script_name..".lua"

        local event = runEvent(event_id, script_path)
        if event == nil then
            player:send_message(script_path.." was not found!")
        end

        return event
    elseif event_type == HANDLER_TYPE_SHOP then
        return runEvent(event_id, "events/generic/GilShopkeeper.lua")
    elseif event_type == HANDLER_TYPE_WARP then
        local warp_name = GAME_DATA:get_warp_logic_name(event_id)

        if warp_name == '' then
            return runEvent(event_id, "events/generic/Warp.lua")
        else
            local script_path = "events/warp/"..warp_name..".lua"
            local event = runEvent(event_id, script_path)
            if event == nil then
                player:send_message(script_path.." was not found!")
            end

            return event
        end
    elseif event_type == HANDLER_TYPE_AETHERYTE then
        --- The Aetheryte sheet actually begins at 0, not 327680
        local aetheryte_id = event_id & 0xFFF

        --- Aetherytes and Aethernet shards are handled by different event scripts
        if GAME_DATA:is_aetheryte(aetheryte_id) then
            return runEvent(event_id, "events/generic/Aetheryte.lua")
        else
            return runEvent(event_id, "events/generic/AethernetShard.lua")
        end
    elseif event_type == HANDLER_TYPE_GUILD_LEVE_ASSIGNMENT then
        return runEvent(event_id, "events/generic/Levemete.lua")
    elseif event_type == HANDLER_TYPE_DEFAULT_TALK then
        return runEvent(event_id, "events/generic/DefaultTalk.lua")
    elseif event_type == HANDLER_TYPE_CUSTOM_TALK then
        local script_name = GAME_DATA:get_custom_talk_name(event_id)
        local script_id = extractScriptId(script_name)
        local script_folder = folderFromScriptId(script_id)
        local script_path = "events/custom/"..script_folder.."/"..script_name..".lua"

        local event = runEvent(event_id, script_path)
        if event == nil then
            player:send_message(script_path.." was not found!")
        end

        return event
    elseif event_type == HANDLER_TYPE_GIMMICK_ACCESSOR then
        return runEvent(event_id, "events/generic/GimmickAccessor.lua")
    elseif event_type == HANDLER_TYPE_GIMMICK_BILL then
        return runEvent(event_id, "events/generic/GimmickBill.lua")
    elseif event_type == HANDLER_TYPE_CHOCOBO_TAXI_STAND then
        return runEvent(event_id, "events/generic/Chocobokeep.lua")
    elseif event_type == HANDLER_TYPE_OPENING then
        local script_name = GAME_DATA:get_opening_name(event_id)
        return runEvent(event_id, "events/quest/opening/"..script_name..".lua")
    elseif event_type == HANDLER_TYPE_EXIT_RANGE then
        return runEvent(event_id, "events/generic/ExitRange.lua")
    elseif event_type == HANDLER_TYPE_SPECIAL_SHOP then
        return runEvent(event_id, "events/generic/SpecialShop.lua")
    elseif event_type == HANDLER_TYPE_SWITCH_TALK then
        return runEvent(event_id, "events/generic/SwitchTalk.lua")
    elseif event_type == HANDLER_TYPE_GOLD_SAUCER_ARCADE_MACHINE then
        return runEvent(event_id, "events/generic/GoldSaucerArcadeMachine.lua")
    elseif event_type == HANDLER_TYPE_GOLD_SAUCER_TALK then
        return runEvent(event_id, "events/generic/GoldSaucerTalk.lua")
    elseif event_type == HANDLER_TYPE_TOPIC_SELECT then
        return runEvent(event_id, "events/generic/TopicSelect.lua")
    elseif event_type == HANDLER_TYPE_PRE_HANDLER then
        return runEvent(event_id, "events/generic/PreHandler.lua")
    elseif event_type == HANDLER_TYPE_DESCRIPTION then
        return runEvent(event_id, "events/generic/Description.lua")
    elseif event_type == HANDLER_TYPE_INCLUSION_SHOP then
        return runEvent(event_id, "events/generic/InclusionShop.lua")
    elseif event_type == HANDLER_TYPE_COLLECTABLES_SHOP then
        return runEvent(event_id, "events/generic/CollectablesShop.lua")
    elseif event_type == HANDLER_TYPE_EVENT_GIMMICK_PATH_MOVE then
        return runEvent(event_id, "events/generic/GimmickPathMove.lua")
    end

    -- Directors
    -- TODO: support more than just dungeons
    if event_type == HANDLER_TYPE_INSTANCE_CONTENT then
        local content_id = event_id & 0xFF
        local short_name = GAME_DATA:get_content_short_name(content_id)
        local script_path = "content/"..short_name..".lua"

        local event = runEvent(event_id, script_path)
        if event == nil then
            player:send_message(script_path.." was not found!")
        end

        return event
    end

    return nil
end
