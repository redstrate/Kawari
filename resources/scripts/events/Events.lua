-- Please keep these ids sorted in each table!

-- Basically, all Event NPCs reference a specific sheet. You can see this in ENpcData in the ENpcBase sheet.
-- A non-exhaustive list of sheets referenced are as follows, which correspond to what that Event NPC does:

-- ChocoboTaxiStand
-- CollectablesShop
-- ContentNpc
-- CraftLeve
-- CustomTalk
-- DefaultTalk
-- DisposalShop
-- DpsChallengeOfficer
-- EventPathMove
-- FccShop
-- GCShop
-- GilShop
-- GuildOrderGuide
-- GuildOrderOfficer
-- GuildleveAssignment
-- InclusionShop
-- LotteryExchangeShop
-- PreHandler
-- Quest
-- SpecialShop
-- Story
-- SwitchTalk
-- TopicSelect
-- TripleTriad
-- Warp

-- Each "section" of ids appear to be broken up into segments of 65536 (which is also the max of a unsigned 16-bit integer)
SECTION_SIZE = 65536

-- This allows us (and probably the client as well) to determine which event belongs to each sheet, or type of NPC.
-- Here they are, sorted:
EVENT_TYPE_QUESTS = 1
EVENT_TYPE_WARP = 2
EVENT_TYPE_GIL_SHOP = 4
EVENT_TYPE_AETHERYTE = 5
EVENT_TYPE_GUILD_LEVE_ASSIGNMENT = 6
EVENT_TYPE_DEFAULT_TALK = 9
EVENT_TYPE_CUSTOM_TALK = 11
EVENT_TYPE_CRAFT_LEVEL = 14
EVENT_TYPE_CHOCOBO_TAXI_STAND = 18
EVENT_TYPE_GC_SHOP = 22
EVENT_TYPE_GUILD_ORDER_GUIDE = 23
EVENT_TYPE_GUILD_ORDER_OFFICER = 24
EVENT_TYPE_CONTENT_NPC = 25
EVENT_TYPE_STORY = 26
EVENT_TYPE_SPECIAL_SHOP = 27
EVENT_TYPE_SWITCH_TALK = 31
EVENT_TYPE_TRIPLE_TRIAD = 35
EVENT_TYPE_GOLD_SAUCER_ARCADE_MACHINE = 36 -- See GoldSaucerArcadeMachine Excel sheet
EVENT_TYPE_FCC_SHOP = 42
EVENT_TYPE_DPS_CHALLENGE_OFFICER = 47
EVENT_TYPE_TOPIC_SELECT = 50
EVENT_TYPE_LOTTERY_EXCHANGE_SHOP = 52
EVENT_TYPE_DISPOSAL_SHOP = 53
EVENT_TYPE_PRE_HANDLER = 54
EVENT_TYPE_INCLUSION_SHOP = 58
EVENT_TYPE_COLLECTABLES_SHOP = 59
EVENT_TYPE_EVENT_PATH_MOVE = 61
EVENT_TYPE_EVENT_GIMMICK_PATH_MOVE = 64 -- These are used for the Solution Nine teleporter pads, for example

-- TODO: Should probably break misc. events and their tables off into separate NPCs and objects eventually, but this is fine for now.
to_sort = {
    [720898] = "DeliveryMoogle.lua",
    [721096] = "ToyChest.lua",
    [721028] = "UnendingJourney.lua",
    [721044] = "CrystalBell.lua",
    [721098] = "HuntBoard.lua",
    [721226] = "Orchestrion.lua",
    [721347] = "GlamourDresser.lua",
    [721440] = "SummoningBell.lua",
    [720935] = "MarketBoard.lua",
    [720978] = "Armoire.lua",
    [1179657] = "Chocobokeep.lua", -- Chocobokeep in Central Shroud
}

-- Events in /common that aren't already covered by other tables
common_events = {
    [720915] = "GenericMender.lua",
    [721480] = "GenericGemstoneTrader.lua", -- Generic Shadowbringers in-city gemstone traders
    [721479] = "GenericGemstoneTrader.lua", -- Generic Shadowbringers per-zone gemstone traders
    -- [721619] = "GenericGemstoneTrader.lua", -- Generic Endwalker & Dawntrail per-zone gemstone traders, but they do nothing when interacted with right now
    -- [721620] = "GenericGemstoneTrader.lua", -- Generic Endwalker & Dawntrail in-city gemstone traders, but they do nothing when interacted with right now
}

-- Not custom in the sense of non-SQEX content, just going based off the directory name
custom0_events = {
    [720901] = "RegFstAdvGuild_00005.lua",
    [720905] = "CmnDefRetainerDesk_00009.lua",
    [720916] = "cmndefinnbed_00020.lua",
}

custom1_events = {
    [721001] = "CmnGscWeeklyLotUnlockTalk_00105.lua",
    [721044] = "cmndefbeautysalon_00148.lua",
    [721052] = "RegFstCarlineCanopy_00156.lua",
}

custom2_events = {
    [721122] = "CmnGscDailyLotDescription_00226.lua",
    [721138] = "CmnGscGATENotice_00242.lua",
}

-- Events in quests/*
quests = {
    [1245185] = "OpeningLimsaLominsa.lua",
    [1245186] = "OpeningGridania.lua",
    [1245187] = "OpeningUldah.lua", 
}

COMMON_DIR = "events/common/"
TOSORT_DIR = "events/tosort/"
OPENING_DIR = "events/quest/opening/"
CUSTOM0_DIR = "events/custom/000/"
CUSTOM1_DIR = "events/custom/001/"
CUSTOM2_DIR = "events/custom/002/"

-- This is called whenever the client requests to start an event
function dispatchEvent(event_id, game_data)
    local event_type = event_id >> 16
    if event_type == EVENT_TYPE_GIL_SHOP then
        return runEvent(event_id, "events/common/GilShopkeeper.lua")
    elseif event_type == EVENT_TYPE_WARP then
        local warp_name = game_data:get_warp_logic_name(event_id)
        -- TODO: don't hardcode all named warps to inns, there's also rental chocobos and more
        -- (see WarpLogic Excel sheet)
        if warp_name == '' then
            return runEvent(event_id, "events/common/GenericWarp.lua")
        else
            return runEvent(event_id, "events/warp/WarpInnGeneric.lua")
        end
    elseif event_type == EVENT_TYPE_AETHERYTE then
        --- The Aetheryte sheet actually begins at 0, not 327680
        local aetheryte_id = event_id & 0xFFF

        --- Aetherytes and Aethernet shards are handled by different event scripts
        if game_data:is_aetheryte(aetheryte_id) then
            return runEvent(event_id, "events/common/GenericAetheryte.lua")
        else
            return runEvent(event_id, "events/common/GenericAethernetShard.lua")
        end
    elseif event_type == EVENT_TYPE_GUILD_LEVE_ASSIGNMENT then
        return runEvent(event_id, "events/common/GenericLevemete.lua")
    elseif event_type == EVENT_TYPE_SPECIAL_SHOP then
        return runEvent(event_id, "events/common/GenericHuntCurrencyExchange.lua") --TODO: Should probably rename this since it now covers other generic currency vendors like Gold Saucer ones
    elseif event_type == EVENT_TYPE_TOPIC_SELECT then
        return runEvent(event_id, "events/common/GenericTopicSelect.lua")
    elseif event_type == EVENT_TYPE_EVENT_GIMMICK_PATH_MOVE then
        return runEvent(event_id, "events/walkin_trigger/SolutionNineTeleporter.lua")
    end

    return runLegacyEvent(event_id)
end

-- everything else
for event_id, script_file in pairs(to_sort) do
    registerEvent(event_id, TOSORT_DIR..script_file)
end

for event_id, script_file in pairs(common_events) do
    registerEvent(event_id, COMMON_DIR..script_file)
end

for event_id, script_file in pairs(custom0_events) do
    registerEvent(event_id, CUSTOM0_DIR..script_file)
end

for event_id, script_file in pairs(custom1_events) do
    registerEvent(event_id, CUSTOM1_DIR..script_file)
end

for event_id, script_file in pairs(custom2_events) do
    registerEvent(event_id, CUSTOM2_DIR..script_file)
end

for event_id, script_file in pairs(quests) do
    registerEvent(event_id, OPENING_DIR..script_file)
end
