-- Basically, all Event NPCs reference a specific sheet. You can see this in ENpcData in the ENpcBase sheet.
-- Events are then run through the dispatcher, which references either a generic script or a custom one.

-- This allows us (and probably the client as well) to determine which event belongs to each sheet, or type of NPC.
-- Here they are, sorted:
EVENT_TYPE_QUESTS = 1 -- See Quests Excel sheet
EVENT_TYPE_WARP = 2 -- See Warp Excel sheet
EVENT_TYPE_GIL_SHOP = 4 -- See GilShop Excel sheet
EVENT_TYPE_AETHERYTE = 5 -- See Aetheryte Excel sheet
EVENT_TYPE_GUILD_LEVE_ASSIGNMENT = 6 -- See GuildleveAssignment Excel sheet
EVENT_TYPE_DEFAULT_TALK = 9 -- See DefaultTalk Excel sheet
EVENT_TYPE_CUSTOM_TALK = 11 -- See CustomTalk Excel sheet
EVENT_TYPE_CRAFT_LEVEL = 14 -- See CraftLeve Excel sheet
EVENT_TYPE_CHOCOBO_TAXI_STAND = 18 -- See ChocoboTaxiStand Excel sheet
EVENT_TYPE_OPENING = 19 -- See Opening Excel sheet
EVENT_TYPE_GC_SHOP = 22 -- See GCShop Excel sheet
EVENT_TYPE_GUILD_ORDER_GUIDE = 23 -- See GuildOrderGuide Excel sheet
EVENT_TYPE_GUILD_ORDER_OFFICER = 24 -- See GuildOrderOfficer Excel sheet
EVENT_TYPE_CONTENT_NPC = 25 -- See ContentNpc Excel sheet
EVENT_TYPE_STORY = 26 -- See Story Excel sheet
EVENT_TYPE_SPECIAL_SHOP = 27 -- See SpecialShop Excel sheet
EVENT_TYPE_SWITCH_TALK = 31 -- See SwitchTalk Excel sheet
EVENT_TYPE_TRIPLE_TRIAD = 35 -- See TripleTriad Excel sheet
EVENT_TYPE_GOLD_SAUCER_ARCADE_MACHINE = 36 -- See GoldSaucerArcadeMachine Excel sheet
EVENT_TYPE_FCC_SHOP = 42 -- See FccShop Excel sheet
EVENT_TYPE_DPS_CHALLENGE_OFFICER = 47 -- See DpsChallengeOfficer Excel sheet
EVENT_TYPE_TOPIC_SELECT = 50 -- See TopicSelect Excel sheet
EVENT_TYPE_LOTTERY_EXCHANGE_SHOP = 52 -- See LotteryExchangeShop Excel sheet
EVENT_TYPE_DISPOSAL_SHOP = 53 -- See DisposalShop Excel sheet
EVENT_TYPE_PRE_HANDLER = 54 -- See PreHandler Excel sheet
EVENT_TYPE_INCLUSION_SHOP = 58 -- See InclusionShop Excel sheet
EVENT_TYPE_COLLECTABLES_SHOP = 59 -- See CollectablesShop Excel sheet
EVENT_TYPE_EVENT_PATH_MOVE = 61 -- See EventPathMove Excel sheet
EVENT_TYPE_EVENT_GIMMICK_PATH_MOVE = 64 -- These are used for the Solution Nine teleporter pads, for example. See EventGimmickPathMove Excel sheet

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

    if event_type == EVENT_TYPE_QUESTS then
        local script_name = GAME_DATA:get_quest_name(event_id)
        local script_id = extractScriptId(script_name)
        local script_folder = folderFromScriptId(script_id)
        local script_path = "events/quest/"..script_folder.."/"..script_name..".lua"

        local event = runEvent(event_id, script_path)
        if event == nil then
            player:send_message(script_path.." was not found!")
        end

        return event
    elseif event_type == EVENT_TYPE_GIL_SHOP then
        return runEvent(event_id, "events/generic/GilShopkeeper.lua")
    elseif event_type == EVENT_TYPE_WARP then
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
    elseif event_type == EVENT_TYPE_AETHERYTE then
        --- The Aetheryte sheet actually begins at 0, not 327680
        local aetheryte_id = event_id & 0xFFF

        --- Aetherytes and Aethernet shards are handled by different event scripts
        if GAME_DATA:is_aetheryte(aetheryte_id) then
            return runEvent(event_id, "events/generic/Aetheryte.lua")
        else
            return runEvent(event_id, "events/generic/AethernetShard.lua")
        end
    elseif event_type == EVENT_TYPE_GUILD_LEVE_ASSIGNMENT then
        return runEvent(event_id, "events/generic/Levemete.lua")
    elseif event_type == EVENT_TYPE_DEFAULT_TALK then
        return runEvent(event_id, "events/generic/DefaultTalk.lua")
    elseif event_type == EVENT_TYPE_CUSTOM_TALK then
        local script_name = GAME_DATA:get_custom_talk_name(event_id)
        local script_id = extractScriptId(script_name)
        local script_folder = folderFromScriptId(script_id)
        local script_path = "events/custom/"..script_folder.."/"..script_name..".lua"

        local event = runEvent(event_id, script_path)
        if event == nil then
            player:send_message(script_path.." was not found!")
        end

        return event
    elseif event_type == EVENT_TYPE_CHOCOBO_TAXI_STAND then
        return runEvent(event_id, "events/generic/Chocobokeep.lua")
    elseif event_type == EVENT_TYPE_OPENING then
        local script_name = GAME_DATA:get_opening_name(event_id)
        return runEvent(event_id, "events/quest/opening/"..script_name..".lua")
    elseif event_type == EVENT_TYPE_SPECIAL_SHOP then
        return runEvent(event_id, "events/generic/SpecialShop.lua")
    elseif event_type == EVENT_TYPE_SWITCH_TALK then
        return runEvent(event_id, "events/generic/SwitchTalk.lua")
    elseif event_type == EVENT_TYPE_TOPIC_SELECT then
        return runEvent(event_id, "events/generic/TopicSelect.lua")
    elseif event_type == EVENT_TYPE_PRE_HANDLER then
        -- TODO: check the quest requirement for these/
        local target_event_id = GAME_DATA:get_pre_handler_target(event_id)
        return dispatchEvent(player, target_event_id)
    elseif event_type == EVENT_TYPE_EVENT_GIMMICK_PATH_MOVE then
        return runEvent(event_id, "events/generic/GimmickPathMove.lua")
    end

    return nil
end
