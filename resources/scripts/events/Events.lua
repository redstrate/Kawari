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

generic_warps = {
    131077,  -- Ferry Skipper from Old Gridania to East Shroud: Sweetbloom Pier
    131078,  -- Ferry Skipper from East Shroud: Sweetbloom Pier to Old Gridania
    131079,  -- Exit Limsa Upper Decks to Mizzenmast Inn room
    131080,  -- Exit New Gridania to The Roost Inn room
    131081,  -- Exit Ul'dah: Steps of Nald to The Hourglass Inn room
    131082,  -- Exit Mizzenmast Inn to Limsa Upper Decks
    131083,  -- Exit The Roost to New Gridania
    131084,  -- Exit The Hourglass to Ul'dah: Steps of Nald
    131086,  -- Ferry Skipper from Western Thanalan: The Silver Bazaar to Western Thanalan: Cescent Cove
    131087,  -- Ferry Skipper from Western Thanalan: Crescent Cove to Western Thanalan: The Silver Bazaar
    131088,  -- Exit from Western Thanalan: Vesper Bay to The Waking Sands
    131089,  -- Exit from The Waking Sands to Western Thanalan: Vesper Bay
    131090,  -- Exit from The Waking Sands to The Solar
    131091,  -- Exit from The Solar to The Waking Sands
    131092,  -- Exit from Limsa Bulwark Hall and/or Drowning Wench to Airship Landing
    131093,  -- Exit from Limsa Bullwark Hall and/or Airship Landing to Drowning Wench
    131094,  -- Exit from Limsa Airship Landing and/or Drowning Wench to Bulwark Hall
    131095,  -- Exit from Ul'dah Hustings Strip and/or Ruby Road Exchange to Airship Landing, these three events get reused in several places to ensure they all connect
    131096,  -- Exit from Ul'dah Airship Landing and/or Ruby Road Exchange to Hustings Strip
    131097,  -- Exit from Ul'dah Airship Landing and/or Husting Strip to Ruby Road Exchange
    131107,  -- Nunuri <Ferry Ticketer> from Western Thanalan: Vesper Bay to Limsa Lominsa: The Lower Decks
    131108,  -- Rhetkympf <Ferry Ticketer> from Limsa Lominsa: The Lower Decks to Western Thanalan: Vesper Bay
    131109,  -- Rerenasu <Ferry Skipper> from Limsa Lominsa: The Lower Decks to Western La Noscea: Aleport
    131110,  -- Ferry Skipper from Western La Noscea: Aleport to Limsa Lominsa: The Lower Decks
    131111,  -- Rerenasu <Ferry Skipper> from Limsa Lominsa: The Lower Decks to Eastern La Noscea: Costa Del Sol
    131112,  -- Ferry Skipper from Eastern La Noscea: Costa Del Sol to Limsa Lominsa: The Lower Decks
    131113,  -- Ferry Skipper from Upper La Noscea: Memeroon's Trading Post to Upper La Noscea: Jijiroon's Trading Post
    131114,  -- Ferry Skipper from Upper La Noscea: Jijiroon's Trading Post to Upper La Noscea: Memeroon's Trading Post
    131115,  -- O'nolosi <Ferry Skipper> from Lower La Noscea: Candlekeep Quay to Western La Noscea: Aleport
    131116,  -- Ferry Skipper from Western La Noscea: Aleport to Lower La Noscea: Candlekeep Quay
    131119,  -- Ferry Skipper from Eastern La Noscea: Hidden Falls Docks to Eastern La Noscea: Raincatcher Gully Docks
    131120,  -- Ferry Skipper from Eastern La Noscea: Raincatcher Gully Docks to Eastern La Noscea: Hidden Falls Docks
    131126,  -- Gatekeeper from Southern Thanalan: Nald's Reflection to Southern Thanalan: The Minotaur Malm
    131131,  -- Ferry Skipper from Moraby Drydocks to Wolves' Den Pier
    131132,  -- Ferry Skipper from Wolves' Den Pier to Moraby Drydocks
    131133,  -- Ferry Skipper from Western La Noscea: The Isles of Umbra to Western La Noscea: Aleport
    131134,  -- Ferry Skipper from Western La Noscea: Aleport to Western La Noscea: The Isles of Umbra
    -- 131158, None -- Ferry Skipper from Old Gridania to The Lavender Beds, needs special handling for housing
    -- 131160, None -- Rerenasu <Ferry Skipper> from Limsa Lominsa: The Lower Decks to Mist, needs special handling for housing
    131169,  -- Ferry Skipper from Eastern La Noscea: Costa Del Sol to ELN: Rhotano Privateer
    131177,  -- Exit from The Gold Saucer (Lift Operator) to The Gold Saucer: Chocobo Square
    131178,  -- Exit from The Gold Saucer: Chocobo Square (Lift Operator) to The Gold Saucer
    131192,  -- House Fortemps Guard <Gatekeep> From Ishgard: The Pillars to Fortemps Manor
    131195,  -- Exit from Fortemps manor to Ishgard: The Pillars
    131204,  -- Exit Ishgard: Foundation to Cloud Nine Inn room
    131205,  -- Exit Cloud Nine to Ishgard: Foundation
    131245,  -- Exit Kugane to Bokairo Inn room
    131246,  -- Exit Bokairo Inn to Kugane
    -- 131248,  -- Kimachi <Ferry Skipper> from Kugane to Shirogane, needs special handling for housing
    131250,  -- Gatekeeper from The Fringes: Castrum Oriens to East Shroud: Amarissaaix's Spire
    131251,  -- Gatekeeper from East Shroud: Amarissaaix's Spire to The Fringes: Castrum Oriens
    131252,  -- Uguisu <Ferry Skipper> from Kugane to Limsa Lominsa: The Lower Decks
    131253,  -- East Aldenard Trading Company Sailor from Limsa Lominsa: The Lower Decks to Kugane
    131255,  -- Ala Mhigan Resistance Gate Guard from The Fringes: Virdjala to The Fringes: Pike Falls
    131266,  -- Gatekeeper from The House of the Fierce to dead-end cave (unable to dive currently)
    131268,  -- Enclave Skiff Captain from The Doman Enclave to Yanxia: The Glittering Basin
    131299,  -- Ala Mhigan Resistance Gate Guard from The Fringes: Pike Falls to The Fringes: Virdjala
    131312,  -- Exit The Pendants Personal Suite to Crystarium
    131313,  -- Exit from The Crown Lift (Lift Operator) to Eulmore: The Canopy
    131390,  -- Exit via Pawlin <Dreamer's Run Doorman> from Old Gridania: Dreamer's Run (old Hatchingtide event area which is now out of bounds) to Old Gridania: Botanists' Guild
    131402,  -- Exit Andron to Old Sharlayan
    131405,  -- Aergwynt <Ferry Ticketer> from Old Sharlayan to Limsa Lominsa: The Lower Decks
    131406,  -- Sailor <Ferryman> from Limsa Lominsa: The Lower Decks to Old Sharlayan
    131428,  -- Exit from The Mothercrystal to Labyrinthos: The Aitiascope (outside, on overworld): probably supposed to drop you into a cutscene zone instead.
    131519,  -- Faire Adventurer from Eastern La Noscea: bottom of the Moonfire Festival (2023 tower to the first tier of the tower
    131545,  -- Port Official from Tuliyollal to Old Sharlayan
    131578,  -- Exit The For'ard Cabins to Tuliyollal
    131609,  -- Exit from The Ageless Necropolis to Living Memory: The Meso Terminal
}

generic_inns = {
    131079, -- Exit Limsa Upper Decks to Mizzenmast Inn room
    131080, -- Exit New Gridania to The Roost Inn room
    131081, -- Exit Ul'dah: Steps of Nald to The Hourglass Inn room
    131204, -- Exit Ishgard: Foundation to Cloud Nine Inn room
    131245, -- Exit Kugane to Bokairo Inn room
    131316, -- Exit from The Crystarium to The Pendants Personal Suite
    131401, -- Exit from Old Sharlayan to The Andron
    131576, -- Exit from Tuliyollal to The For'ard Cabins
}


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
    [720916] = "cmndefinnbed_00020.lua",
}

custom1_events = {
    [721044] = "cmndefbeautysalon_00148.lua",
    [721052] = "RegFstCarlineCanopy_00156.lua",
}

-- Events in quests/*
quests = {
    [1245185] = "OpeningLimsaLominsa.lua",
    [1245186] = "OpeningGridania.lua",
    [1245187] = "OpeningUldah.lua", 
}

COMMON_DIR = "events/common/"
WARP_DIR = "events/warp/"
TOSORT_DIR = "events/tosort/"
OPENING_DIR = "events/quest/opening/"
CUSTOM0_DIR = "events/custom/000/"
CUSTOM1_DIR = "events/custom/001/"
TRIGGER_DIR = "events/walkin_trigger/"

-- This is called whenever the client requests to start an event
function dispatchEvent(event_id, game_data)
    local event_type = event_id >> 16
    if event_type == EVENT_TYPE_AETHERYTE then
        --- The Aetheryte sheet actually begins at 0, not 327680
        local aetheryte_id = event_id & 0xFFF

        --- Aetherytes and Aethernet shards are handled by different event scripts
        if game_data:is_aetheryte(aetheryte_id) then
            return runEvent(event_id, "events/common/GenericAetheryte.lua")
        else
            return runEvent(event_id, "events/common/GenericAethernetShard.lua")
        end
    elseif event_type == EVENT_TYPE_GIL_SHOP then
        return runEvent(event_id, "events/common/GilShopkeeper.lua")
    elseif event_type == EVENT_TYPE_GUILD_LEVE_ASSIGNMENT then
        return runEvent(event_id, "events/common/GenericLevemete.lua")
    elseif event_type == EVENT_TYPE_SPECIAL_SHOP then
        return runEvent(event_id, "events/common/GenericHuntCurrencyExchange.lua") --TODO: Should probably rename this since it now covers other generic currency vendors like Gold Saucer ones
    elseif event_type == EVENT_TYPE_EVENT_GIMMICK_PATH_MOVE then
        return runEvent(event_id, "events/walkin_trigger/SolutionNineTeleporter.lua")
    end

    return runLegacyEvent(event_id)
end

-- everything else
for _, event_id in pairs(generic_warps) do
    registerEvent(event_id, "events/common/GenericWarp.lua")
end

for _, event_id in pairs(generic_inns) do
    registerEvent(event_id, "events/warp/WarpInnGeneric.lua" )
end

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

for event_id, script_file in pairs(quests) do
    registerEvent(event_id, OPENING_DIR..script_file)
end
