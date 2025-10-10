-- Please keep these ids sorted in each table!

-- Basically, all Event NPCs reference a specific sheet. You can see this in ENpcData in the ENpcBase sheet.

-- Each "section" of ids appear to be broken up into segments of 65536 (which is also the max of a unsigned 16-bit integer)
SECTION_SIZE = 65536

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

-- TODO: Should probably break misc. events and their tables off into separate NPCs and objects eventually, but this is fine for now.
to_sort = {
    [720935] = "MarketBoard.lua",
    [1179657] = "Chocobokeep.lua", -- Chocobokeep in Central Shroud
}

-- Events in /common that aren't already covered by other tables
common_events = {
    [721480] = "GemstoneTrader.lua", -- Generic Shadowbringers in-city gemstone traders
    [721479] = "GemstoneTrader.lua", -- Generic Shadowbringers per-zone gemstone traders
    -- [721619] = "GenericGemstoneTrader.lua", -- Generic Endwalker & Dawntrail per-zone gemstone traders, but they do nothing when interacted with right now
    -- [721620] = "GenericGemstoneTrader.lua", -- Generic Endwalker & Dawntrail in-city gemstone traders, but they do nothing when interacted with right now
}

-- Not custom in the sense of non-SQEX content, just going based off the directory name
custom0_events = {
    [720898] = "CmnDefMogLetter_00002.lua",
    [720901] = "RegFstAdvGuild_00005.lua",
    [720905] = "CmnDefRetainerDesk_00009.lua",
    [720915] = "CmnDefNpcRepair_00019.lua",
    [720916] = "CmnDefInnBed_00020.lua",
    [720978] = "CmnDefCabinet_00082.lua",
}

custom1_events = {
    [721001] = "CmnGscWeeklyLotUnlockTalk_00105.lua",
    [721028] = "CmnDefCutSceneReplay_00132.lua",
    [721044] = "CmnDefBeautySalon_00148.lua",
    [721052] = "RegFstCarlineCanopy_00156.lua",
}

custom2_events = {
    [721096] = "CmnDefMiniGame_00200.lua",
    [721098] = "ComDefMobHuntBoard_00202.lua",
    [721122] = "CmnGscDailyLotDescription_00226.lua",
    [721138] = "CmnGscGATENotice_00242.lua",
}

custom3_events = {
    [721226] = "HouFurOrchestrion_00330.lua",
}

custom4_events = {
    [721347] = "CmnDefPrismBox_00451.lua",
}

custom5_events = {
    [721440] = "CmnDefRetainerBell_00544.lua",
}

-- Events in quests/*
quests = {
    [1245185] = "OpeningLimsaLominsa.lua",
    [1245186] = "OpeningGridania.lua",
    [1245187] = "OpeningUldah.lua", 
}

GENERIC_DIR = "events/generic/"
TOSORT_DIR = "events/tosort/"
OPENING_DIR = "events/quest/opening/"
CUSTOM0_DIR = "events/custom/000/"
CUSTOM1_DIR = "events/custom/001/"
CUSTOM2_DIR = "events/custom/002/"
CUSTOM3_DIR = "events/custom/003/"
CUSTOM4_DIR = "events/custom/004/"
CUSTOM5_DIR = "events/custom/005/"

-- This is called whenever the client requests to start an event
function dispatchEvent(event_id, game_data)
    local event_type = event_id >> 16
    if event_type == EVENT_TYPE_GIL_SHOP then
        return runEvent(event_id, "events/generic/GilShopkeeper.lua")
    elseif event_type == EVENT_TYPE_WARP then
        local warp_name = game_data:get_warp_logic_name(event_id)
        -- TODO: don't hardcode all named warps to inns, there's also rental chocobos and more
        -- (see WarpLogic Excel sheet)
        if warp_name == '' then
            return runEvent(event_id, "events/generic/Warp.lua")
        else
            return runEvent(event_id, "events/warp/WarpInnGeneric.lua")
        end
    elseif event_type == EVENT_TYPE_AETHERYTE then
        --- The Aetheryte sheet actually begins at 0, not 327680
        local aetheryte_id = event_id & 0xFFF

        --- Aetherytes and Aethernet shards are handled by different event scripts
        if game_data:is_aetheryte(aetheryte_id) then
            return runEvent(event_id, "events/generic/Aetheryte.lua")
        else
            return runEvent(event_id, "events/generic/AethernetShard.lua")
        end
    elseif event_type == EVENT_TYPE_GUILD_LEVE_ASSIGNMENT then
        return runEvent(event_id, "events/generic/Levemete.lua")
    elseif event_type == EVENT_TYPE_SPECIAL_SHOP then
        return runEvent(event_id, "events/generic/SpecialShop.lua")
    elseif event_type == EVENT_TYPE_TOPIC_SELECT then
        return runEvent(event_id, "events/generic/TopicSelect.lua")
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
    registerEvent(event_id, GENERIC_DIR..script_file)
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

for event_id, script_file in pairs(custom3_events) do
    registerEvent(event_id, CUSTOM3_DIR..script_file)
end

for event_id, script_file in pairs(custom4_events) do
    registerEvent(event_id, CUSTOM4_DIR..script_file)
end

for event_id, script_file in pairs(custom5_events) do
    registerEvent(event_id, CUSTOM5_DIR..script_file)
end

for event_id, script_file in pairs(quests) do
    registerEvent(event_id, OPENING_DIR..script_file)
end
